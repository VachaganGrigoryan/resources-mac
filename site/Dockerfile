# Project website image. Build from the repo root: docker build -f site/Dockerfile .
# Only site/ and the app screenshots and icon enter the build (see /.dockerignore).
FROM rust:1-slim-bookworm AS build
WORKDIR /src
COPY site/Cargo.toml site/Cargo.lock ./
COPY site/src ./src
RUN cargo build --release --locked

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /src/target/release/resources-site /resources-site
COPY site/static /srv/static
COPY docs/screenshots /srv/static/screenshots
COPY docs/icon.png /srv/static/icon.png
ENV SITE_DIR=/srv/static SITE_ADDR=0.0.0.0:8080
EXPOSE 8080
ENTRYPOINT ["/resources-site"]
