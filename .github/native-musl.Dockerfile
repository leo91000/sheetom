FROM rust:1.88-alpine

RUN apk add --no-cache build-base nodejs npm \
  && npm install --global npm@11.16.0
