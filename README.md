# Unipept API
![codecov](https://img.shields.io/codecov/c/github/unipept/unipept-api/develop)

This package is an implementation of the Unipept API that's being used by the Unipept Web Application and Desktop application to succesfully perform analysis of metaproteomics samples.

## Overview of existing endpoints
This is an exhaustive list of all endpoints that are exposed by this API

### Public endpoints
#### API v1
* `/api/v1/pept2taxa`
* `/api/v1/pept2lca`
* `/api/v1/taxa2lca`
* `/api/v1/pept2prot`
* `/api/v1/pept2funct`
* `/api/v1/pept2ec`
* `/api/v1/pept2go`
* `/api/v1/pept2interpro`
* `/api/v1/taxa2tree`
* `/api/v1/peptinfo`
* `/api/v1/taxonomy`
* `/api/v1/messages`

#### API v2
* `/api/v2/pept2taxa`
* `/api/v2/pept2lca`
* `/api/v2/taxa2lca`
* `/api/v2/pept2prot`
* `/api/v2/pept2funct`
* `/api/v2/pept2ec`
* `/api/v2/pept2go`
* `/api/v2/pept2interpro`
* `/api/v2/taxa2tree`
* `/api/v2/peptinfo`
* `/api/v2/taxonomy`
* `/api/v2/messages`

### Private endpoints
* `/private_api/goterms`
* `/private_api/ecnumbers`
* `/private_api/interpros`
* `/private_api/taxa`
* `/private_api/proteins`
* `/private_api/metadata`
* `/mpa/pept2data`
* `/datasets/sampledata`

## Developing the Unipept API
You can use the included devcontainer in order to start working on this API.
The devcontainer will automatically download the most recent version of the Unipept Index built from SwissProt.
Follow these steps in order to easily work on the Unipept API in the devcontainer:

* You first have to build the binaries by running `cargo build --release`.
* Then, you should start the mariadb server: `sudo service mariadb start`
* Sometimes the password for the database user has not been initialized properly, open a MySQL shell and execute this command: `alter user 'root'@'localhost' identified by 'root_pass'; flush privileges;`.
* Finally, the Unipept API can be started with this command: `./target/release/unipept-api -i "/unipept-index-data" -d "mysql://root:root_pass@localhost:3306/unipept" -p 80`.
