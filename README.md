# Remote – Cloud Instance Manager
Simple CLI tool for managing cloud instances for remote development.


Allows you to configure multiple remote instances with different cloud provider credential profiles. You can store
configurations and swap between them (your "active" instance) for quick interaction.


Currently only supports AWS EC2 instances, but I will add more cloud providers as I need.

## Install
Compile and install via cargo
```
cargo install --git https://github.com/kykosic/remote.git
```

## Usage
* Configure a new remote instance
```
remote new
```
* Switch active remote instance
```
remote instance [alias]
```
* Get active instance status
```
remote status
```
* Start active instance
```
remote start
```
* Stop active instance
```
remote stop
```
* SSH into active instance (optional port forwards)
```
remote ssh [-p 8888] [-p 8080]
```
* Download file from active instance
```
remote download /path/to/remote.file /path/to/local.file
```
* Upload file to active instance
```
remote upload /path/to/local.file /path/to/remote.file
```
* Set active instance type
```
remote resize [instance-type]
```
* List available instances (optional cloud/profile)
```
remote ls [cloud] [profile]
```
* Remove a remote instance
```
remote rm [alias]
```
