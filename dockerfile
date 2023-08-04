FROM rust:1.70.0-bullseye as builder
WORKDIR /app
COPY . .
RUN cargo install --path .


FROM debian:bullseye-20230703-slim as runner
COPY --from=builder /usr/local/cargo/bin/wallet /usr/local/bin/wallet
ENV ROCKET_ADDRESS=0.0.0.0
EXPOSE 8000
CMD ["wallet"]

# https://www.koyeb.com/tutorials/deploy-a-rust-web-app-with-rocket