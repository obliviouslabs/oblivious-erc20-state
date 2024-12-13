cd docker
mkdir external
cd external
git clone ssh://git@github.com/obliviouslabs/ordb.git
git clone ssh://git@github.com/xtrm0/verified_contract_state.git
cd ../../
docker build -t oblivious_erc20_state .

# After just run:
# docker-compose up