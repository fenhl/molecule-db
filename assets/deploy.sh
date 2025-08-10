#!/bin/sh

set -e

git push
ssh fenhl.net rustup update stable
ssh fenhl.net cargo install-update -g molecule-db
ssh fenhl.net env -C /opt/git/github.com/fenhl/molecule-db/main git pull
ssh fenhl.net sudo systemctl restart molecule-db
