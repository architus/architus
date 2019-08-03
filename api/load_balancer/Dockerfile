# Use an official Python runtime as a parent image
FROM python:3.6

# Set the working directory to /app
WORKDIR /app

# Copy the current directory contents into the container at /app
COPY . /app

# Install any needed packages specified in requirements.txt
RUN pip install --trusted-host pypi.python.org -r requirements.txt

EXPOSE 7300
EXPOSE 7200
EXPOSE 6300
EXPOSE 6200

# Run app.py when the container launches
CMD ["python", "-u", "forwarder_device.py"]
