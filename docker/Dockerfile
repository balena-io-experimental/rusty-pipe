FROM rust as builder

WORKDIR /build
RUN mkdir -p /build/src && \
    echo "fn main() { println!(\"Rebuild Me\"); }" > /build/src/main.rs
COPY Cargo.* ./
RUN cargo build

RUN rm -f /build/src/main.rs
COPY src ./src
RUN touch src/main.rs && sleep 1 && cargo build

FROM debian

ENV TINI_VERSION v0.18.0
ADD https://github.com/krallin/tini/releases/download/${TINI_VERSION}/tini /tini
RUN chmod +x /tini
ENTRYPOINT ["/tini", "--"]

COPY --from=builder /build/target/debug/rusty-pipe /usr/bin/rusty-pipe

CMD [ "rusty-pipe", "--help"]