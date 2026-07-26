FROM pytorch/pytorch:2.6.0-cuda12.4-cudnn9-runtime

RUN --mount=type=cache,target=/root/.cache/pip \
    pip install --default-timeout=600 \
    transformers \
    datasets \
    evaluate \
    scikit-learn \
    pandas \
    pyarrow \
    pyyaml \
    onnx \
    onnxruntime \
    optimum \
    accelerate

RUN rm -rf /root/.cache/pip

WORKDIR /workspace

ENTRYPOINT ["python"]
CMD ["training/finbert_sft/train.py"]