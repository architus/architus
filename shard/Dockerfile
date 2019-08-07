# Use an official Python runtime as a parent image
FROM python:3.6


WORKDIR /app
COPY ./shard/requirements.txt /app

# Install any needed packages specified in requirements.txt
RUN pip install --trusted-host pypi.python.org -r requirements.txt


# Copy the current directory contents into the container at /app
COPY  ./shard/ /app/

# Run app.py when the container launches
CMD ["python", "-u", "bot.py"]
