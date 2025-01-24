# For building the DCAP validator
FROM golang:1.22 as go-tdx-builder
WORKDIR /root/
RUN git clone https://github.com/Ruteri/dummy-tdx-dcap
WORKDIR /root/dummy-tdx-dcap
RUN go mod download
RUN CGO_ENABLED=0 GOOS=linux go build -o dcap-verifier cmd/httpserver/main.go

FROM ubuntu:22.04

# Update and install Python
RUN apt-get update && \
    apt-get install -y python3 python3-pip

RUN apt-get install -y curl wget git dumpasn1

# Rust project deps
RUN apt-get install -y clang libclang-dev openssl libssl-dev pkg-config

# Install foundry
WORKDIR /root/
RUN wget https://github.com/foundry-rs/foundry/releases/download/nightly-c3069a50ba18cccfc4e7d5de9b9b388811d9cc7b/foundry_nightly_linux_amd64.tar.gz
RUN tar -xzf ./foundry_nightly_linux_amd64.tar.gz -C /usr/local/bin

# Install the TDX checker
COPY --from=go-tdx-builder /root/dummy-tdx-dcap/dcap-verifier /usr/local/bin

# Install rust and cargo
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"


# Project
WORKDIR /workdir
COPY ./src ./src
COPY ./macros_tests ./macros_tests
COPY ./tests ./tests
COPY ./docker/external ./external
COPY ./benches ./benches
COPY ./.rustfmt.toml .

COPY ./Cargo.lock .
COPY ./Cargo.toml .
COPY ./.env .

COPY ./run.sh .

RUN cat Cargo.toml
RUN ls -la ./external
RUN cargo build --release

# ENTRYPOINT [ ]
CMD [ "bash", "run.sh" ]
# # CMD [ "python", "replicatoor.py" ]
