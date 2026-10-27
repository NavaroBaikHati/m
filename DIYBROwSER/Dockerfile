# Build/uji di lingkungan yang sama dengan mesin uji (Arch Linux).
#
#   docker build -t diybrowser-arch .                  # siapkan dependensi + UI
#   docker run --rm diybrowser-arch cargo test --workspace
#   docker run --rm diybrowser-arch cargo build --release --locked
#   docker run --rm -v "$PWD/docs/screenshots:/app/docs/screenshots" \
#     diybrowser-arch scripts/smoke-gui.sh
#
# Daftar paket di bawah juga berlaku untuk instalasi langsung di Arch
# (lihat README bagian Dependensi Sistem).
FROM archlinux:latest AS dasar

RUN pacman -Syu --noconfirm \
      base-devel \
      rust \
      nodejs \
      npm \
      webkit2gtk-4.1 \
      libsoup3 \
      gtk3 \
      librsvg \
      openssl \
      libayatana-appindicator \
      xdotool \
      xorg-server-xvfb \
      imagemagick \
      python \
      git \
 && pacman -Scc --noconfirm

WORKDIR /app

FROM dasar AS ui

COPY . .

RUN npm --prefix ui ci \
 && npm --prefix ui run build

# Tahap build dipisah supaya `docker build` untuk pengujian cepat tidak perlu
# mengompilasi seluruh workspace.
FROM ui AS build

RUN cargo build --release --locked

CMD ["cargo", "test", "--workspace"]
