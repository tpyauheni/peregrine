#!/bin/bash
mysqladmin ping -h localhost -u "$HEALTHCHECK_USER" -p"$HEALTHCHECK_PASSWORD"