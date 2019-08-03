# Use an official Python runtime as a parent image
FROM python:3.6

WORKDIR /app
RUN pip install websockets

COPY . /app
EXPOSE 5300
ENV NUM_SHARDS=5
CMD ["python", "-u", "app.py"]
