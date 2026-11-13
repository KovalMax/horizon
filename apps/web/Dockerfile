FROM node:26.13.0-trixie-slim
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci
COPY index.html tsconfig.json vite.config.ts ./
COPY public ./public
COPY src ./src
COPY tests ./tests
EXPOSE 3000
CMD ["npm", "run", "dev"]
