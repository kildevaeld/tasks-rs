#!/bin/bash

DEST=tasks-http/examples/tls
openssl req -x509 -nodes -days 365 -newkey rsa:2048 -keyout ${DEST}/key.rsa -out ${DEST}/cert.pem