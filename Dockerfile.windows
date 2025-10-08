FROM mcr.microsoft.com/windows/nanoserver:ltsc2022

ADD ./target/release/ddns_rust.exe C:/app/ddns_rust.exe

ENTRYPOINT ["C:/app/ddns_rust.exe", "run", "--loops"]