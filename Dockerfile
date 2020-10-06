FROM rust:1.46

RUN mkdir /app && mkdir /pdna
WORKDIR /pdna

COPY PowerDNA_Linux_4.10.1.14.tgz /pdna/
RUN tar xzf PowerDNA_Linux_4.10.1.14.tgz \
    && cd /pdna/PowerDNA_4.10.1/src \
    && make \
    && make install

WORKDIR /app

CMD bash
