FROM rust:1.81-alpine as builder
WORKDIR /usr/src/app
COPY src src
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
RUN apk add musl-dev
RUN cargo install --path .

FROM alpine:3
WORKDIR /usr/src/app
COPY --from=builder   /usr/local/cargo/bin/answer-bot /usr/local/bin/answer-bot
CMD ["answer-bot"]


