#!/bin/bash

# Environment variables for SSL certificate
DOMAIN=domain.com
EMAIL=admin@domain.com

# Environment variables for AWS CLI resources
BUCKET=s3-bucket-name
QUEUE_URL=queue-name.fifo
SECRET_ID=secret-id

# Environment variables for Docker
DOCKER_REGISTRY=ghcr.io
DOCKER_USERNAME=username
DOCKER_IMAGE=image-name

echo "Installing AWS CLI"
sudo apt update
sudo apt install -y apt-transport-https ca-certificates curl software-properties-common unzip
curl "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip" -o "awscliv2.zip"
unzip awscliv2.zip
sudo ./aws/install

echo "Installing Docker"
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg
echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable" | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
sudo apt update
sudo apt install -y docker-ce


echo "Configuring Docker"
sudo usermod -aG docker ubuntu

echo "Installing Nginx"
sudo apt install -y nginx

echo "Setting up Nginx"
cat > nginx.conf <<EOF
server {
  server_name $DOMAIN;

  location / {
    proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
    proxy_set_header Host \$host;
    proxy_pass http://127.0.0.1:8080;
    proxy_http_version 1.1;
    proxy_set_header Upgrade \$http_upgrade;
    proxy_set_header Connection "upgrade";
  }
}
server {
  listen 80;
  listen [::]:80;

  server_name $DOMAIN;
  
  if (\$host = $DOMAIN) {
    return 301 https://$DOMAIN\$request_uri;
  }
  return 404;
}
EOF
sudo mv nginx.conf /etc/nginx/sites-available/$DOMAIN
sudo ln -s /etc/nginx/sites-available/$DOMAIN /etc/nginx/sites-enabled/
sudo rm /etc/nginx/sites-enabled/default
sudo systemctl restart nginx

echo "Setting up SSL certificate"
sudo apt install -y certbot python3-certbot-nginx
sudo certbot --nginx --domain $DOMAIN -m $EMAIL --agree-tos --non-interactive --redirect

# Setting up environment variables
echo "Setting up shadow-deploy"
aws s3api get-object --bucket $BUCKET --key scripts/shadow-deploy shadow-deploy
sudo mv shadow-deploy /usr/bin/shadow-deploy
sudo chmod 711 /usr/bin/shadow-deploy
cat > shadow-deploy.conf <<EOF
cwd: /home/ubuntu
queue_name: $QUEUE_URL
commands:
  - docker login $DOCKER_REGISTRY -u $DOCKER_USERNAME -p {{secrets.$SECRET_ID.DOCKER_PASSWORD}}
  - docker pull $DOCKER_REGISTRY/$DOCKER_USERNAME/$DOCKER_IMAGE:{{message}}
  - docker container rename webapp webapp-old || true
  - docker stop webapp-old || true
  - "docker run -d --name webapp -p 8080:8080 \
      -e DB={{secrets.$SECRET_ID.DB}} \
      $DOCKER_REGISTRY/$DOCKER_USERNAME/$DOCKER_IMAGE:{{message}}"
  - docker container rm webapp-old || true

EOF
sudo mv shadow-deploy.conf /etc/shadow-deploy.conf

echo "Creating shadow-deploy service"
cat > shadow-deploy.service <<EOF
[Unit]
Description=Shadow Deploy Service
After=network.target

[Service]
Type=simple
ExecStart=/usr/bin/shadow-deploy
Restart=on-failure
RestartSec=10
StandardOutput=file:/var/log/shadow-deploy.log
StandardError=file:/var/log/shadow-deploy.log

[Install]
WantedBy=multi-user.target
EOF
sudo mv shadow-deploy.service /etc/systemd/system/shadow-deploy.service
sudo systemctl daemon-reload
sudo systemctl enable shadow-deploy.service
sudo systemctl start shadow-deploy.service

echo "Setting up webapp"
aws sqs send-message --queue-url $QUEUE_URL --message-body master --message-group-id shadow-deploy

echo "Cleaning up"
rm -r aws*

echo "Done"