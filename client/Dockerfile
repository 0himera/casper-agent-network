FROM node:20-alpine

WORKDIR /usr/src/app

COPY package*.json ./
# Use npm install with legacy peer deps to tolerate peer-dependency conflicts and lockfile drift
RUN npm install --legacy-peer-deps

COPY . .

EXPOSE 5173

CMD ["npm", "run", "dev", "--", "--host"]
