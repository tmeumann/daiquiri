FROM ubuntu:20.04

RUN apt-get update && apt-get -y install build-essential

RUN mkdir /app && mkdir /pdna && mkdir /etc/daiquiri
WORKDIR /pdna

COPY PowerDNA_Linux_4.10.1.14.tgz /pdna/
RUN tar xzf PowerDNA_Linux_4.10.1.14.tgz \
    && cd /pdna/PowerDNA_4.10.1/src \
    && make \
    && make install

WORKDIR /app

COPY streams.json /etc/daiquiri/streams.json
COPY target/x86_64-unknown-linux-gnu/release/daiquiri /app/daiquiri

EXPOSE 3030 5555

CMD ["/app/daiquiri"]
