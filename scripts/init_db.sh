#!/usr/bin/env bash
set -x
set -eo pipefail

if ! [ -x "$(command -v sqlx)" ]; then
	echo >&2 "Error: sqlx is not installed."
	exit 1
fi

DB_PORT="${DB_PORT:=5432}"
SUPERUSER="${SUPERUSER:=postgres}"
SUPERUSER_PWD="${SUPERUSER_PWD:=password}"
APP_USER="${APP_USER:=app}"
APP_USER_PWD="${APP_USER_PWD:=secret}"
APP_DB_NAME="${APP_DB_NAME:=thread}"

if [[ -z "${SKIP_DOCKER}" ]]; then
	RUNNING_POSTGRES_CONTAINER=$(docker ps --filter 'name=postgres' --format '{{.ID}}')
	if [[ -n $RUNNING_POSTGRES_CONTAINER ]]; then
		echo >&2 "A postgres container is already running, kill it with"
		echo >&2 "    'docker kill ${RUNNING_POSTGRES_CONTAINER}'."
		exit 1
	fi
	CONTAINER_NAME="postgres_$(date '+%s')"
	docker run \
		--env POSTGRES_USER=${SUPERUSER} \
		--env POSTGRES_PASSWORD=${SUPERUSER_PWD} \
		--health-cmd="pg_isready -U ${SUPERUSER} || exit 1" \
		--health-interval=1s \
		--health-timeout=5s \
		--health-retries=5 \
		--publish "${DB_PORT}":5432 \
		--detach \
		--name "${CONTAINER_NAME}" \
		postgres -N 1000

	until [ \
		"$(docker inspect -f "{{.State.Health.Status}}" ${CONTAINER_NAME})" == \
		"healthy" \
		]; do
		echo >&2 "Postgres is still unavailable - sleeping"
		sleep 1
	done

	CREATE_QUERY="CREATE USER ${APP_USER} WITH PASSWORD '${APP_USER_PWD}';"
	docker exec -it "${CONTAINER_NAME}" psql -U "${SUPERUSER}" -c "${CREATE_QUERY}"

	GRANT_QUERY="ALTER USER ${APP_USER} CREATEDB;"
	docker exec -it "${CONTAINER_NAME}" psql -U "${SUPERUSER}" -c "${GRANT_QUERY}"
fi

echo >&2 "Postgres is up and running on port ${DB_PORT} - running migrations now!"

DATABASE_URL=postgres://${APP_USER}:${APP_USER_PWD}@localhost:${DB_PORT}/${APP_DB_NAME}
export DATABASE_URL
sqlx database create
sqlx migrate run

echo >&2 "Postgres has been migrated, ready to go!"
