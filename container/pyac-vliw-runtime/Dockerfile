FROM rust:1.99.1-bookworm AS vliw-build

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates build-essential libvulkan-dev spirv-tools \
  && rm -rf /var/lib/apt/lists/* \
  && rustup toolchain install nightly-2026-08-15 --profile minimal --component rust-src \
  && cargo install --git https://github.com/rust-gpu/rust-gpu --tag v0.11.0 cargo-gpu --locked

WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml /src/
COPY crates/pyac /src/crates/pyac
COPY crates/pyac-vliw-core /src/crates/pyac-vliw-core
COPY crates/gpu/shader /src/crates/gpu/shader
COPY crates/gpu/cuda-kernel /src/crates/gpu/cuda-kernel

RUN cargo gpu build --auto-install-rust-toolchain \
    --shader-crate /src/crates/gpu/shader \
    --target spirv-unknown-spv1.5 \
    --features vulkan-device-atomics,shader-f64 \
    --capabilities Float64 --capabilities Int64 \
    --output-dir /src/crates/gpu/target/generated/vulkan
RUN spirv-val --target-env vulkan1.2 /src/crates/gpu/target/generated/vulkan/pyac_vliw_gpu_kernel.spv

COPY crates/gpu/worker /src/crates/gpu/worker
RUN cargo build --release \
    --manifest-path /src/crates/gpu/worker/Cargo.toml \
    --bin pyac-vliw-gpu-worker \
    --bin pyac-vliw-runtime \
    --bin pyac-vliw-pipeline-compiler

FROM ubuntu:24.04

ENV PYAC_VLIW_RUNTIME_HOST=0.0.0.0 \
    PYAC_VLIW_RUNTIME_PORT=18887 \
    PYAC_VLIW_WORKER_PATH=/service/pyac-vliw-gpu-worker \
    PYAC_VLIW_SPIRV_PATH=/service/pyac_vliw_gpu_kernel.spv \
    PYAC_VLIW_PIPELINE_CACHE_PATH=/service/artifacts/pyac_vliw_pipeline_cache.bin \
    NVIDIA_DRIVER_CAPABILITIES=compute,utility,graphics \
    VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates libegl1 libvulkan1 libx11-6 libxext6 \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /service
COPY --from=vliw-build /src/target/release/pyac-vliw-gpu-worker /service/pyac-vliw-gpu-worker
COPY --from=vliw-build /src/target/release/pyac-vliw-runtime /service/pyac-vliw-runtime
COPY --from=vliw-build /src/target/release/pyac-vliw-pipeline-compiler /service/pyac-vliw-pipeline-compiler
COPY --from=vliw-build /src/crates/gpu/target/generated/vulkan/pyac_vliw_gpu_kernel.spv /service/pyac_vliw_gpu_kernel.spv

EXPOSE 18887
ENTRYPOINT ["/service/pyac-vliw-runtime"]
