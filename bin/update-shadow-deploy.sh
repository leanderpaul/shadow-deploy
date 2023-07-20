#!/bin/bash

# Environment variables for AWS CLI resources
BUCKET=s3-bucket-name

aws s3api get-object --bucket $BUCKET --key scripts/shadow-deploy shadow-deploy
sudo mv shadow-deploy /usr/bin/shadow-deploy
sudo chmod 711 /usr/bin/shadow-deploy
sudo systemctl restart shadow-deploy.service