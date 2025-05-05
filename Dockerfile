FROM rust:bookworm

# Install dependencies
RUN apt-get update --fix-missing
RUN apt-get install -y pkg-config openssl libssl-dev git sudo curl python3 python3-pip
RUN cargo install cargo-make
RUN curl -Ls https://raw.githubusercontent.com/extism/python-pdk/main/install.sh > extism.sh
RUN chmod +x extism.sh
RUN ./extism.sh
RUN curl https://get.extism.org/cli > extism.sh
RUN ./extism.sh -y
RUN curl https://raw.githubusercontent.com/extism/js-pdk/main/install.sh > extism.sh
RUN chmod +x extism.sh
RUN ./extism.sh
RUN rm extism.sh
RUN apt-get update -y
RUN apt-get upgrade -y --fix-missing