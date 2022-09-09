FROM docker:dind

RUN apk add --no-cache curl pkgconfig openssl-dev gcc musl-dev
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
