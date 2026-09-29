FROM python:3.11-slim

WORKDIR /app

# OpenSSL 3 disables MD4 by default; ntlm-auth requires it for NTLMv2 proofs.
RUN apt-get update \
    && apt-get install -y --no-install-recommends openssl \
    && printf '\nopenssl_conf = openssl_init\n\n[openssl_init]\nproviders = provider_sect\n\n[provider_sect]\ndefault = default_sect\nlegacy = legacy_sect\n\n[default_sect]\nactivate = 1\n\n[legacy_sect]\nactivate = 1\n' >> /etc/ssl/openssl.cnf \
    && rm -rf /var/lib/apt/lists/*

ENV OPENSSL_CONF=/etc/ssl/openssl.cnf

COPY requirements.txt server.py golden_selftest.py ./
RUN pip install --no-cache-dir -r requirements.txt

RUN mkdir -p /evidence /golden

ENV TDS_MOCK_HOST=0.0.0.0
ENV TDS_MOCK_PORT=1433
ENV TDS_MOCK_DOMAIN=APPFW
ENV TDS_MOCK_USERNAME=svc-app
ENV TDS_MOCK_PASSWORD=AppSvc!Passw0rd
ENV TDS_MOCK_EVIDENCE=/evidence/ntlm-handshake.json

EXPOSE 1433

CMD ["python", "server.py"]
