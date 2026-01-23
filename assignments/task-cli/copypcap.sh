#!/bin/bash

VM_USER="davidlynch"
VM_IP="192.168.64.2"
REMOTE_DIR="/home/davidlynch"

scp "$VM_USER@$VM_IP:$REMOTE_DIR/*.pcap" .
