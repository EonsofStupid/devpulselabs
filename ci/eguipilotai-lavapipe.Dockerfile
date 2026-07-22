# Ubuntu 24.04 multi-architecture index, pinned so hosted-runner image churn
# cannot silently change the EguiPilotAI software-rendering environment.
FROM ubuntu:24.04@sha256:4fbb8e6a8395de5a7550b33509421a2bafbc0aab6c06ba2cef9ebffbc7092d90

ENV DEBIAN_FRONTEND=noninteractive \
    LANG=C.UTF-8 \
    LC_ALL=C.UTF-8 \
    TZ=UTC \
    WGPU_BACKEND=vulkan \
    LIBGL_ALWAYS_SOFTWARE=true \
    VK_DRIVER_FILES=/usr/share/vulkan/icd.d/lvp_icd.x86_64.json \
    RUSTUP_HOME=/opt/rustup \
    CARGO_HOME=/opt/cargo \
    PATH=/opt/cargo/bin:${PATH}

RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential ca-certificates clang cmake curl fontconfig \
        fonts-dejavu-core git libasound2-dev libegl1-mesa-dev \
        libfontconfig1-dev libudev-dev \
        libvulkan1 libwayland-dev libx11-xcb-dev libxkbcommon-dev \
        libxcursor-dev libxi-dev libxinerama-dev libxrandr-dev \
        mesa-vulkan-drivers pkg-config xauth xvfb \
    && rm -rf /var/lib/apt/lists/* \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --profile minimal --default-toolchain stable \
    && rustup component add rustfmt clippy \
    && rustup target add wasm32-unknown-unknown \
    && fc-cache -f

WORKDIR /workspace
