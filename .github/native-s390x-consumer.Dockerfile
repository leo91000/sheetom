FROM --platform=$BUILDPLATFORM debian:bookworm-slim AS node-download

ARG NODE_VERSION=22.23.2
ARG NODE_SHA256=50aa0935c7caee2f95434f84c2c19f16e4f223257eadb341a3a2d5aaa545bbe6

RUN apt-get update \
  && apt-get install --yes --no-install-recommends ca-certificates curl xz-utils \
  && curl --fail --location --silent --show-error \
    --output /tmp/node.tar.xz \
    "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-linux-s390x.tar.xz" \
  && echo "${NODE_SHA256}  /tmp/node.tar.xz" | sha256sum --check \
  && mkdir /node \
  && tar --extract --xz --file /tmp/node.tar.xz --strip-components=1 --directory /node

FROM ubuntu:22.04

COPY --from=node-download /node/ /usr/local/

ENTRYPOINT ["node"]
