FROM archlinux:base-devel AS builder

RUN pacman -Syu --noconfirm --needed \
    git \
    rustup \
    clang \
    openssl \
    pkgconf \
    cmake \
    mold \
    && rustup default nightly

RUN git clone https://github.com/tpyauheni/peregrine.git /app
WORKDIR /app

RUN cargo build --release --features server --bin web

FROM archlinux:base

RUN pacman -Syu --noconfirm --needed \
    openssl \
    ca-certificates \
    && pacman -Scc --noconfirm

COPY --from=builder /app/target/release/web /usr/local/bin/

EXPOSE 8080

CMD ["web"]