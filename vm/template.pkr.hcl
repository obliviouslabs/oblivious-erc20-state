packer {
  required_plugins {
    qemu = {
      version = ">= 1.0.0"
      source  = "github.com/hashicorp/qemu"
    }
  }
}

source "qemu" "ubuntu" {
  accelerator   = "kvm"
  iso_url       = "https://cloud-images.ubuntu.com/noble/20241120/noble-server-cloudimg-amd64.img"
  iso_checksum  = "sha256:c35d76a81cc2209352f39ec653c91f0ce919c51d3a69a655c8e78e76722334b6"
  output_directory = "output"
  cd_files         = ["./cloud-init/*"]
  cd_label         = "cidata"

  disk_size        = "64G"
  disk_compression = true
  disk_image       = true
  headless         = true

  shutdown_command = "echo 'packer' | sudo -S shutdown -P now"
  qemuargs = [
    ["-m", "8192M"],
    ["-smp", "4"],
    ["-serial", "mon:stdio"],
  ]

  ssh_password     = "123456"
  ssh_username     = "ubuntu"
  ssh_timeout = "30m"
}

build {
  sources = ["source.qemu.ubuntu"]

  provisioner "shell-local" {
    inline = [
      "$(cd $(git rev-parse --show-toplevel) && cargo build --release)",
      "cp $(git rev-parse --show-toplevel)/target/release/oblivious_erc20_state .",
    ]
  }
  
  provisioner "file" {
    source     = "oblivious_erc20_state"
    destination = "/home/ubuntu/oblivious_erc20_state"
  }

  provisioner "shell" {
    // run scripts with sudo, as the default cloud image user is unprivileged
    execute_command = "echo 'packer' | sudo -S sh -c '{{ .Vars }} {{ .Path }}'"
    // NOTE: cleanup.sh should always be run last, as this performs post-install cleanup tasks
    scripts = [
      "scripts/setup.sh",
      "scripts/cleanup.sh"
    ]
  }
}
