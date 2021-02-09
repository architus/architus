FROM openjdk:12-alpine

WORKDIR /app
COPY ./lavalink /app
RUN wget https://github.com/Frederikam/Lavalink/releases/download/3.3.2.3/Lavalink.jar

CMD java -jar Lavalink.jar
