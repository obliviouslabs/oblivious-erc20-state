#!/bin/bash -eux

echo "==> waiting for cloud-init to finish"
while [ ! -f /var/lib/cloud/instance/boot-finished ]; do
    echo 'Waiting for Cloud-Init...'
    sleep 1
done

echo "==> updating apt cache"
sudo apt-get update -qq

echo "==> upgrade apt packages"
sudo apt-get upgrade -y -qq

echo "==> installing qemu-guest-agent"
sudo apt-get install -y -qq qemu-guest-agent

echo "==> install tdx-guest"
git clone https://github.com/canonical/tdx.git --branch noble-24.04
cd tdx
sed -i 's/TDX_SETUP_ATTESTATION=0/TDX_SETUP_ATTESTATION=1/' ./setup-tdx-config
./setup-tdx-guest.sh