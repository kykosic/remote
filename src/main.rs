use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::string::ToString;

use anyhow::{Error, Result};
use dirs::home_dir;
use remote::{AwsCloud, Cloud, InstanceConfig, InstanceManager, ProfileConfig};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(
    name = "remote",
    author = "Kyle Kosic <kylekosic@gmail.com>",
    about = "Simple CLI for managing remote instances"
)]
enum Opt {
    #[structopt(about = "Set the active instance")]
    Instance {
        /// The alias of the instance as set in "new"
        alias: String,
    },
    #[structopt(about = "Configure a new instance")]
    New {
        /// If specified, will set this as the active instance
        #[structopt(short, long)]
        active: bool,
    },
    #[structopt(about = "Remove an instance by alias")]
    Rm {
        /// Alias of instance to remove
        alias: String,
    },
    #[structopt(about = "Start active instance")]
    Start,
    #[structopt(about = "Stop active instance")]
    Stop,
    #[structopt(about = "Get status of active instance")]
    Status,
    #[structopt(about = "SSH into the active instance")]
    Ssh,
    #[structopt(about = "Change the type of the active instance")]
    Resize {
        /// The desired instance type
        instance_type: String,
    },
    #[structopt(about = "List configured instances or available instances for a cloud profile")]
    Ls {
        /// The cloud provider to use
        cloud: Option<String>,
        /// The profile name to use
        #[structopt(default_value = "default")]
        profile: String,
    },
}

fn user_input(prompt: &str) -> Result<String> {
    print!("{}: ", prompt);
    std::io::stdout().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

fn expand_tilde<P: AsRef<Path>>(path_user_input: P) -> Option<PathBuf> {
    let p = path_user_input.as_ref();
    if !p.starts_with("~") {
        return Some(p.to_path_buf());
    }
    if p == Path::new("~") {
        return home_dir();
    }
    dirs::home_dir().map(|mut h| {
        if h == Path::new("/") {
            p.strip_prefix("~").unwrap().to_path_buf()
        } else {
            h.push(p.strip_prefix("~/").unwrap());
            h
        }
    })
}

fn set_active_instance(alias: &str) -> Result<()> {
    let mut config = ProfileConfig::get_or_create()?;
    let names = config
        .instances
        .iter()
        .map(|inst| inst.alias.clone())
        .collect::<Vec<String>>();
    if !names.contains(&alias.to_string()) {
        Err(Error::msg(format!(
            "No instance with alias '{}' found, you may need to create it first",
            alias
        )))?;
    };
    config.active = Some(alias.to_string());
    config.update()?;
    println!("Active instance: {}", alias);
    Ok(())
}

async fn new_instance(set_active: bool) -> Result<()> {
    let mut config = ProfileConfig::get_or_create()?;

    let cloud = user_input("Cloud provider")?;
    let cloud = Cloud::from_str(&cloud)?;
    let mut profile = user_input("Cloud profile [default]")?;
    if profile.as_str() == "" {
        profile = "default".to_string();
    };
    let instance_id = user_input("Instance ID")?;
    let key_path = user_input("SSH key path")?;
    let path = expand_tilde(&key_path).unwrap();
    if !path.exists() {
        Err(Error::msg(format!("Could not find key file: {}", key_path)))?;
    };
    let user = user_input("SSH user name")?;
    let alias = user_input("Alias")?;
    println!("---");

    let duplicates = config
        .instances
        .iter()
        .filter(|inst| inst.alias == alias)
        .collect::<Vec<_>>();
    if duplicates.len() > 0 {
        Err(Error::msg(format!(
            "Instance with alias '{}' already exists",
            alias
        )))?
    };

    let instance = InstanceConfig {
        alias: alias.clone(),
        instance_id,
        key_path,
        user,
        profile,
        cloud,
    };
    status(&instance).await?;

    config.instances.push(instance);
    if set_active {
        config.active = Some(alias)
    }

    config.update()?;
    Ok(())
}

fn remove_instance(alias: &str) -> Result<()> {
    let mut config = ProfileConfig::get_or_create()?;
    config.instances = config
        .instances
        .into_iter()
        .filter(|inst| inst.alias != alias)
        .collect();
    if config.active == Some(alias.to_string()) {
        config.active = None
    };
    config.update()?;
    println!("Removed instance: {}", alias);
    Ok(())
}

fn get_manager(cloud: &Cloud, profile: &str) -> Result<Box<dyn InstanceManager>> {
    match cloud {
        Cloud::Aws => Ok(Box::new(AwsCloud::from_profile(profile)?)),
    }
}

fn get_active_instance() -> Result<InstanceConfig> {
    let config = ProfileConfig::get_or_create()?;
    let active = config.active.ok_or(Error::msg("No active instance"))?;
    let instances = config
        .instances
        .into_iter()
        .filter(|x| x.alias == active)
        .collect::<Vec<InstanceConfig>>();
    if instances.len() == 0 {
        Err(Error::msg(format!(
            "Active instance '{}' not found in instance list",
            active
        )))?;
    };
    Ok(instances[0].to_owned())
}

fn instance_list() -> Result<()> {
    let config = ProfileConfig::get_or_create()?;
    let info = config
        .instances
        .into_iter()
        .map(|inst| inst.to_string())
        .collect::<Vec<_>>()
        .join("\n---\n");
    println!("Configured instances:\n---\n{}", info);
    Ok(())
}

async fn start_instance() -> Result<()> {
    let instance = get_active_instance()?;
    let manager = get_manager(&instance.cloud, &instance.profile)?;
    let state = manager.start_instance(&instance.instance_id).await?;
    println!(
        "{} ({}): {} -> {}",
        instance.alias, instance.instance_id, state.previous, state.current
    );
    Ok(())
}

async fn stop_instance() -> Result<()> {
    let instance = get_active_instance()?;
    let manager = get_manager(&instance.cloud, &instance.profile)?;
    let state = manager.stop_instance(&instance.instance_id).await?;
    println!(
        "{} ({}): {} -> {}",
        instance.alias, instance.instance_id, state.previous, state.current
    );
    Ok(())
}

async fn open_ssh() -> Result<()> {
    let instance = get_active_instance()?;
    let manager = get_manager(&instance.cloud, &instance.profile)?;
    let status = manager.get_instance(&instance.instance_id).await?;
    if status.state.as_str() != "running" {
        Err(Error::msg("Instance is not running"))?;
    };
    if status.public_dns.as_str() == "" {
        Err(Error::msg("Instance has no public DNS"))?;
    };
    let key_path = expand_tilde(&instance.key_path).ok_or(Error::msg(format!(
        "Could not locate key {}",
        &instance.key_path
    )))?;
    let addr = format!("{}@{}", instance.user, status.public_dns);
    let _ = Command::new("ssh")
        .arg("-i")
        .arg(key_path)
        .arg(addr)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .stdin(Stdio::inherit())
        .output()
        .unwrap();
    Ok(())
}

async fn instance_status() -> Result<()> {
    let instance = get_active_instance()?;
    status(&instance).await
}

async fn status(instance: &InstanceConfig) -> Result<()> {
    let manager = get_manager(&instance.cloud, &instance.profile)?;
    let status = manager.get_instance(&instance.instance_id).await?;
    println!("{}", status.to_string());
    Ok(())
}

async fn instance_resize(instance_type: &str) -> Result<()> {
    let instance = get_active_instance()?;
    let manager = get_manager(&instance.cloud, &instance.profile)?;
    manager
        .set_instance_type(&instance.instance_id, instance_type)
        .await?;
    println!(
        "Set {} ({}) to {}",
        instance.alias, instance.instance_id, instance_type
    );
    Ok(())
}

async fn instance_list_cloud(cloud: &str, profile: &str) -> Result<()> {
    let cl = Cloud::from_str(cloud)?;
    let manager = get_manager(&cl, profile)?;
    let instances = manager
        .list_instances()
        .await?
        .into_iter()
        .map(|inst| inst.to_string())
        .collect::<Vec<String>>()
        .join("\n---\n");
    println!("Instances on {} ({}):\n---\n{}", cloud, profile, instances);
    Ok(())
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();
    match opt {
        Opt::Instance { alias } => set_active_instance(&alias)?,
        Opt::New { active } => new_instance(active).await?,
        Opt::Rm { alias } => remove_instance(&alias)?,
        Opt::Start => start_instance().await?,
        Opt::Stop => stop_instance().await?,
        Opt::Ssh => open_ssh().await?,
        Opt::Status => instance_status().await?,
        Opt::Resize { instance_type } => instance_resize(&instance_type).await?,
        Opt::Ls { cloud, profile } => match cloud {
            Some(cloud) => instance_list_cloud(&cloud, &profile).await?,
            None => instance_list()?,
        },
    };
    Ok(())
}
