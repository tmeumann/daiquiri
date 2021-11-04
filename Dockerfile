FROM ubuntu:20.04

RUN apt update && apt -y install build-essential && apt clean

RUN mkdir /app && mkdir /pdna && mkdir /etc/daiquiri
WORKDIR /pdna

COPY PowerDNA_Linux_4.10.1.14.tgz /pdna/
RUN tar xzf PowerDNA_Linux_4.10.1.14.tgz \
    && cd /pdna/PowerDNA_4.10.1/src \
    && make \
    && make install

WORKDIR /app

COPY streams.json /etc/daiquiri/streams.json
COPY target/release/daiquiri /app/daiquiri

EXPOSE 3030 5555

CMD ["/app/daiquiri"]
