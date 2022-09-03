### How certificates were generated

#### `monero_client.crt` and `monero_client.key`

```sh
openssl req -newkey rsa:4096 -nodes -keyout monero_client.key -x509 -days 365 -out monero_client.crt -sha256 -batch
```

and its fingerprint with `openssl x509 -noout -fingerprint -sha256 -inform pem -in monero_client.crt | cut -d "=" -f 2`

#### `monero_client_joined.pem`

```sh
cat monero_client.crt > monero_client_joined.pem && \
    cat monero_client.key >> monero_client_joined.pem
```

#### `monero_client.pfx`

```sh
openssl pkcs12 -export -out monero_client.pfx -inkey monero_client.key -in monero_client.crt
```

#### `monero_gen_server.crt` and `monero_gen_server.key`

```sh
monero-gen-ssl-cert --certificate-filename monero_gen_server.crt --private-key-filename monero_gen_server.key
```

#### `monerod_gen_server.pfx`, used only to make `client` tests fail

```sh
openssl pkcs12 -export -out monero_gen_server.pfx -inkey monero_gen_server.key -in monero_gen_server.crt
```

#### `monero_openssl_no_cn_server.crt` and `monero_openssl_no_cn_server.key`

```sh
openssl req \
    -newkey rsa:4096 -nodes -keyout monero_openssl_no_cn_server.key \
    -x509 -days 365 \
    -out monero_openssl_no_cn_server.crt \
    -sha256 -batch
```

#### `monero_openssl_with_cn_server.crt` and `monero_openssl_with_cn_server.key`

```sh
openssl req \
    -newkey rsa:4096 -nodes -keyout monero_openssl_with_cn_server.key \
    -x509 -days 365 \
    -out monero_openssl_with_cn_server.crt \
    -sha256 -subj '/CN=localhost'
```
