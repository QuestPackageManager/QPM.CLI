#!/bin/bash
apt update
apt upgrade -y
apt install -y \
    libssl-dev \
    pkg-config