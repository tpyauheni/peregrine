CREATE USER 'healthchecker'@'%' IDENTIFIED BY '${HEALTHCHECK_PASSWORD}';
GRANT USAGE ON *.* TO 'healthchecker'@'%';
FLUSH PRIVILEGES;