FROM ubuntu:22.04

RUN mkdir -p /opt/db/

WORKDIR /opt

ADD x64 /opt/x64
ADD phi /opt/phi
ADD phi-prod.toml /opt/phi-prod.toml

RUN chmod + /opt/phi

CMD /opt/phi