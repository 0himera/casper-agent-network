FROM node:20-alpine

WORKDIR /usr/src/app

COPY package*.json ./
RUN npm ci

COPY . .

# Expose the API port (4000)
EXPOSE 4000

# Start the event handler daemon
CMD ["npm", "run", "event-handler:dev"]
