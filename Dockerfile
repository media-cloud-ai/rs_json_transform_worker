FROM rust:1.41-stretch as builder

ADD . /src
WORKDIR /src

RUN apt-get update && \
    apt install -y \
        gcc \
        make \
        autotools-dev \
        jq

ENV JQ_LIB_DIR=/usr/bin/jq

RUN cargo build --verbose --release && \
    cargo install --path .

FROM debian:stretch
COPY --from=builder /usr/local/cargo/bin/rs_json_transform_worker /usr/bin

RUN apt update && \
    apt install -y \
        libssl1.1 \
        ca-certificates
        
ENV AMQP_QUEUE=job_json_transform
CMD rs_json_transform_worker