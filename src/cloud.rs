use std::string::ToString;

use anyhow::{Error, Result};
use async_trait::async_trait;
use rusoto_core::{HttpClient, Region};
use rusoto_credential::{ChainProvider, ProfileProvider};
use rusoto_ec2::{
    filter, AttributeValue, DescribeInstancesRequest, Ec2Client, ModifyInstanceAttributeRequest,
    StartInstancesRequest, StopInstancesRequest,
};

pub use rusoto_ec2::Ec2;

#[derive(Debug, Clone)]
pub struct Instance {
    pub instance_type: String,
    pub instance_id: String,
    pub public_dns: String,
    pub tags: Vec<InstanceTag>,
    pub state: String,
}

impl ToString for Instance {
    fn to_string(&self) -> String {
        let tag_string = self
            .tags
            .iter()
            .map(|tag| format!("\"{}\"=\"{}\"", tag.key, tag.value))
            .collect::<Vec<String>>()
            .join(", ");
        format!(
            "Instance ID: {}\n\
             Type: {}\n\
             Tags: {}\n\
             State: {}",
            self.instance_id, self.instance_type, tag_string, self.state
        )
    }
}

#[derive(Debug, Clone)]
pub struct InstanceTag {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct StateChange {
    pub previous: String,
    pub current: String,
}

#[async_trait]
pub trait InstanceManager {
    async fn list_instances(&self) -> Result<Vec<Instance>>;
    async fn get_instance(&self, instance_id: &str) -> Result<Instance>;
    async fn start_instance(&self, instance_id: &str) -> Result<StateChange>;
    async fn stop_instance(&self, instance_id: &str) -> Result<StateChange>;
    async fn set_instance_type(&self, instance_id: &str, instance_type: &str) -> Result<()>;
}

pub struct AwsCloud {
    client: Ec2Client,
}

impl AwsCloud {
    pub fn new(client: Ec2Client) -> Self {
        Self { client }
    }

    pub fn from_profile(profile: &str) -> Result<Self> {
        let mut provider = ProfileProvider::new()?;
        provider.set_profile(profile);
        let client = Ec2Client::new_with(
            HttpClient::new()?,
            ChainProvider::with_profile_provider(provider),
            Region::default(),
        );
        Ok(Self::new(client))
    }

    async fn describe_instances(&self, instance_id: Option<&str>) -> Result<Vec<Instance>> {
        let filters = match instance_id {
            Some(id) => {
                let filter = filter!("instance-id", id);
                Some(vec![filter])
            }
            None => None,
        };
        let req = DescribeInstancesRequest {
            filters,
            ..Default::default()
        };
        let res: Vec<Instance> = self
            .client
            .describe_instances(req)
            .await?
            .reservations
            .unwrap()
            .into_iter()
            .map(|res| res.instances.unwrap())
            .flatten()
            .map(|inst| Instance {
                instance_type: inst.instance_type.unwrap(),
                instance_id: inst.instance_id.unwrap(),
                public_dns: inst.public_dns_name.unwrap(),
                state: inst.state.unwrap().name.unwrap(),
                tags: inst
                    .tags
                    .unwrap()
                    .into_iter()
                    .map(|tag| InstanceTag {
                        key: tag.key.unwrap(),
                        value: tag.value.unwrap(),
                    })
                    .collect(),
            })
            .collect();
        Ok(res)
    }
}

#[async_trait]
impl InstanceManager for AwsCloud {
    async fn list_instances(&self) -> Result<Vec<Instance>> {
        self.describe_instances(None).await
    }

    async fn get_instance(&self, instance_id: &str) -> Result<Instance> {
        let instances = self.describe_instances(Some(instance_id)).await?;
        match instances.len() {
            0 => Err(Error::msg(format!(
                "Could not find instance {}",
                instance_id
            ))),
            _ => Ok(instances[0].clone()),
        }
    }

    async fn start_instance(&self, instance_id: &str) -> Result<StateChange> {
        let req = StartInstancesRequest {
            instance_ids: vec![instance_id.to_string()],
            ..Default::default()
        };
        let res = self
            .client
            .start_instances(req)
            .await?
            .starting_instances
            .unwrap()[0]
            .clone();
        Ok(StateChange {
            previous: res.previous_state.unwrap().name.unwrap(),
            current: res.current_state.unwrap().name.unwrap(),
        })
    }

    async fn stop_instance(&self, instance_id: &str) -> Result<StateChange> {
        let req = StopInstancesRequest {
            instance_ids: vec![instance_id.to_string()],
            ..Default::default()
        };
        let res = self
            .client
            .stop_instances(req)
            .await?
            .stopping_instances
            .unwrap()[0]
            .clone();
        Ok(StateChange {
            previous: res.previous_state.unwrap().name.unwrap(),
            current: res.current_state.unwrap().name.unwrap(),
        })
    }

    async fn set_instance_type(&self, instance_id: &str, instance_type: &str) -> Result<()> {
        let value = AttributeValue {
            value: Some(instance_type.to_string()),
        };
        let req = ModifyInstanceAttributeRequest {
            instance_id: instance_id.to_string(),
            instance_type: Some(value),
            ..Default::default()
        };
        self.client.modify_instance_attribute(req).await?;
        Ok(())
    }
}
