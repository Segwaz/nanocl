# Create Builder image
FROM rust:1.68.0-alpine3.17

# Setup timezone
ARG tz=Europe/Paris


# Install required dependencies
RUN apk add openssl
RUN apk add libpq-dev
RUN apk add gcc
RUN apk add g++
RUN apk add make
RUN apk add tzdata
RUN apk add util-linux
RUN apk add bash

RUN cargo install cargo-watch --locked

RUN mkdir -p /project
WORKDIR /project

ENV TZ=${tz}

LABEL org.opencontainers.image.source https://github.com/nxthat/nanocl
LABEL org.opencontainers.image.description The dev image for nanocl services

ENTRYPOINT ["cargo"]
