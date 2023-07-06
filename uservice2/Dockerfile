FROM rust:1.67.1-bookworm as builder
WORKDIR /home/uservice
COPY . .

RUN cargo build --release
RUN strip target/release/uservice

FROM debian:bookworm-20230612
WORKDIR /home/uservice
COPY --from=builder /home/uservice/target/release/uservice .
COPY --from=builder /home/uservice/hello-world.html .

RUN apt-get update && apt-get install -y iproute2 procps

EXPOSE 7878/tcp
ENTRYPOINT ["./uservice"]
