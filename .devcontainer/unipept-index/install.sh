#!/bin/bash

# Exit immediately if a command exits with a non-zero status
# and ensure failing commands in pipes are also detected
set -euo pipefail

# Function to handle errors and exit gracefully
error_exit() {
    echo "Error: $1"
    exit 1
}

# Function to handle any error that occurs during execution
trap 'error_exit "An unexpected error occurred."' ERR

# Define variables
FEATURE_DIR="/unipept-index-data"
VERSION_OPTION="${VERSION:-latest}"
GITHUB_API_INDEX="https://api.github.com/repos/unipept/unipept-index/releases"
GITHUB_API_DATABASE="https://api.github.com/repos/unipept/unipept-database/releases"

# Create the feature directory if it doesn't exist
mkdir -p "$FEATURE_DIR"

# Function to get releases from GitHub
get_releases() {
    curl -s "$GITHUB_API_INDEX" | jq -r '.[] | .tag_name' || error_exit "Failed to retrieve releases from GitHub."
}

# Function to download and extract the specified version
download_and_extract() {
    local version="$1"
    local zip_file_name="$2"
    local github_url="$3"
    local release_url
    local zip_file

    # Get the release URL for the specific ZIP file
    release_url=$(curl -s "$github_url" | jq -r --arg zip_name "$zip_file_name" --arg date "$version" '.[] | .assets[] | select(.created_at | contains($date)) | select(.name == $zip_name) | .browser_download_url')

    # Check if release URL is found
    if [ -z "$release_url" ]; then
        error_exit "No release found for version $version with the expected ZIP file: $zip_file_name."
    fi

    zip_file="${FEATURE_DIR}/$(basename "$release_url")"
    echo "Downloading $zip_file..."

    # Perform the curl command
    curl -L -o "$zip_file" "$release_url"
    # Check if curl command succeeded
    if [ $? -ne 0 ]; then
        error_exit "Failed to download $zip_file."
    fi

    echo "Extracting $zip_file to $FEATURE_DIR..."

    # Perform the unzip command
    unzip -o "$zip_file" -d "$FEATURE_DIR"
    # Check if unzip command succeeded
    if [ $? -ne 0 ]; then
        error_exit "Failed to extract $zip_file."
    fi

    # Remove the ZIP file
    rm "$zip_file"
    # Check if removal succeeded
    if [ $? -ne 0 ]; then
        error_exit "Failed to remove $zip_file after extraction."
    fi

    # Store the version in the .VERSION file
    echo "$version" > "$FEATURE_DIR/.VERSION" || error_exit "Failed to write version to .VERSION file."
}

# Function to list the last 10 releases
list_last_10_releases() {
    echo "Available releases:"
    get_releases | head -n 10 || error_exit "Failed to list the last 10 releases."
}

# Determine which version to download
download_version() {
    if [ "$VERSION_OPTION" = "latest" ]; then
        # Get the latest version
        latest_version=$(get_releases | head -n 1) || error_exit "Failed to determine the latest version."

        # Extract the date part from the latest version
        latest_version_date=${latest_version#*SP_}; latest_version_date=${latest_version_date%.zip}

        # First, download the zip file containing the index files
        download_and_extract "$latest_version_date" "index_SP_${latest_version_date}.zip" "$GITHUB_API_INDEX"
        # Then, download the zip file containing the database files
        download_and_extract "$latest_version_date" "suffix-array.zip" "$GITHUB_API_DATABASE"
        echo "Successfully downloaded and extracted the latest version: $latest_version_date"
    else
        # Attempt to download the specified version
        download_and_extract "$VERSION_OPTION" "index_SP_${VERSION_OPTION}.zip" "$GITHUB_API_INDEX" || {
            echo "No release available for the specified date: $VERSION_OPTION"
            list_last_10_releases
            exit 1
        }
        download_and_extract "$VERSION_OPTION" "suffix-array.zip" "$GITHUB_API_DATABASE"
        echo "Successfully downloaded and extracted version: $VERSION_OPTION"
    fi
}

DB_TMP_DIR="~/db_schemas/"
DB_USER="root"
DB_PASSWORD="root_pass"

# We also need to install and setup a small MySQL database that requires the UniProt-entries to be loaded in before-
# hand. There's will be used by the Unipept API to retrieve functional annotations and other metadata.
setup_database() {
    echo "Started constructing database..."

    # First, download the database schemas that are required for the suffix array
    mkdir -p "$DB_TMP_DIR"

    # Download the database schema
    wget -q "https://raw.githubusercontent.com/unipept/unipept-database/master/schemas_suffix_array/structure_no_index.sql" -O "$DB_TMP_DIR/structure_no_index.sql"
    # Download an SQL-file that starts building indices for the database
    wget -q "https://raw.githubusercontent.com/unipept/unipept-database/master/schemas_suffix_array/structure_index_only.sql" -O "$DB_TMP_DIR/structure_index_only.sql"

    # Install mariadb-server from apt without user interaction
    export DEBIAN_FRONTEND="noninteractive"
    sudo debconf-set-selections <<< "mariadb-server mysql-server/root_password password $DB_PASSWORD"
    sudo debconf-set-selections <<< "mariadb-server mysql-server/root_password_again password $DB_PASSWORD"

    apt update && apt install -y lz4 mariadb-server

    # Start MariaDB service
    service mariadb start

    # Import the SQL files into the database
    mysql -uroot -p"$DB_PASSWORD" < "$DB_TMP_DIR/structure_no_index.sql"

    # Load the UniProt-entries into the database
    lz4 -dcfm "$FEATURE_DIR/uniprot_entries.tsv.lz4" | mariadb  --local-infile=1 -uroot -p"$DB_PASSWORD" unipept -e "LOAD DATA LOCAL INFILE '/dev/stdin' INTO TABLE uniprot_entries;SHOW WARNINGS" 2>&1

    # Build the database indices
    mysql -uroot -p"$DB_PASSWORD" unipept < "$DB_TMP_DIR/structure_index_only.sql"

    echo "Constructing database finished..."
}

# Correctly move and extract the files required for the datastore used by the Unipept API.
initialize_datastore() {
    mkdir -p "$FEATURE_DIR/datastore"

    # Iterate over each .lz4 file in the source directory
    for lz4_file in "$FEATURE_DIR"/*.tsv.lz4; do
        # Check if the file exists (in case no .lz4 files are present)
        if [[ -f "$lz4_file" ]]; then
            # Extract the file name without the .lz4 extension
            filename=$(basename "$lz4_file" .lz4)

            # Extract the .lz4 file to the target subdirectory
            lz4 -d "$lz4_file" "$FEATURE_DIR/datastore/$filename"

            # Delete original file (we no longer need this)
            rm "$lz4_file"

            echo "Extracted $lz4_file to $FEATURE_DIR/datastore/$filename"
        else
            echo "No .lz4 files found in $FEATURE_DIR"
        fi
    done

    # Download sample data JSON-file (required for the API)
    wget -q "https://raw.githubusercontent.com/unipept/unipept-database/master/schemas_suffix_array/sampledata.json" -O "$FEATURE_DIR/datastore/sampledata.json"

    # Move the .version file to the datastore directory (this file contains the version identifier of UniProt that was used for this database)
    mv "$FEATURE_DIR/.version" "$FEATURE_DIR/datastore/.version"

    # Rename the index file
    mv "$FEATURE_DIR/sa_sparse3_compressed.bin" "$FEATURE_DIR/sa.bin"
}

# Start the setup process
download_version
setup_database
initialize_datastore
