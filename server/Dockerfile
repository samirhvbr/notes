FROM rust:1.98.1-bookworm@sha256:9a73a5088750b4c95158ab26629c854c3d6fc4b173cb7bc8079ad252d8ed7bfa AS build
WORKDIR /src
COPY . .
RUN cargo build --locked --release -p notes-server

FROM debian:bookworm-slim@sha256:88200866dfff7ea7f5cbcb6ec7c8a701889efe6fe859fe64d6990e4b07ea4171
RUN groupadd --gid 10001 notes && useradd --uid 10001 --gid notes --no-create-home notes \
    && mkdir /data && chown notes:notes /data
COPY --from=build /src/target/release/notes-server /usr/local/bin/notes-server
USER 10001:10001
ENV NOTES_SERVER_DATA=/data
VOLUME ["/data"]
ENTRYPOINT ["/usr/local/bin/notes-server"]
CMD ["serve"]
