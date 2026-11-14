#!/bin/bash
apt update
apt upgrade -y
# libdbus is required by the keyring crate on Linux
apt install -y \
    libdbus-1-dev \
    pkg-config
