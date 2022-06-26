FROM rustlang/rust:nightly as build

RUN USER=root cargo new --bin noobchain_server
WORKDIR /noobchain_server

COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock

RUN apt update && apt install -y cmake protobuf-compiler
RUN cargo +nightly build
RUN rm ./src/*.rs

COPY ./src ./src
RUN rm ./target/debug/deps/noobchain_server*
RUN cargo +nightly build

FROM rust:1.61
COPY --from=build /noobchain_server/target/debug/noobchain_server .
ENTRYPOINT RUST_LOG=info ./noobchain_server