FROM python:3.12-slim-bookworm
RUN pip install --no-cache-dir jupyterlab==4.4.* nbconvert clickhouse-connect pandas matplotlib pyyaml
WORKDIR /lab/notebooks
ENTRYPOINT ["jupyter", "lab", "--ip=0.0.0.0", "--port=8888", "--no-browser", "--allow-root", \
            "--IdentityProvider.token=", "--ServerApp.password=", "--ServerApp.root_dir=/lab/notebooks"]
