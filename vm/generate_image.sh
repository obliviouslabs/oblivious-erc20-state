rm -rf build
mkdir build
cp -r ./cloud-init ./build
cp -r ./scripts ./build
cp ./template.pkr.hcl ./build
cd build
$(cd $(git rev-parse --show-toplevel) && cargo build --release)
cp $(git rev-parse --show-toplevel)/target/release/oblivious_erc20_state ./

packer init .
packer validate .
PACKER_LOG=1 packer build .

