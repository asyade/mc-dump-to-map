FROM rustlang/rust:nightly

WORKDIR /app
ENTRYPOINT ["cargo", "build",  "--release" ]