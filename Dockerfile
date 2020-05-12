FROM rust:1.43-stretch as builder

ADD . /src
WORKDIR /src

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
