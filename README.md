# Remote – Cloud Instance Manager
Simple CLI tool for managing cloud instances for remote development.

## Install
Compile and install via cargo
```
cargo install --path .
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
* SSH into active instance
```
remote ssh
```
* Set active instance type
```
remote resize [instance-type]
```
* List configured remote instances
```
remote ls
```
* List available instance for a cloud/profile
```
remote ls [cloud] [profile]
```
* Remove a remote instance
```
remote rm [alias]
```
