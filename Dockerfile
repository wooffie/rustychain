FROM rust:latest AS build

WORKDIR /app

COPY . .

RUN cargo build --release

FROM debian:buster-slim AS run

RUN apt-get update && apt-get install -y libssl-dev

WORKDIR /app

COPY --from=build /app/target/release/main .

ENV DIFFICULTY 00

CMD ["./main"]


# docker build -t wooffie/rustychain:latest .
# docker run -it -e RUST_LOG=info -e DIFFICULTY=bb wooffie/rustychain:latest