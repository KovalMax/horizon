FROM node:26.13.0-trixie-slim
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci
COPY index.html tsconfig.json vite.config.ts ./
COPY public ./public
COPY src ./src
COPY tests ./tests
COPY e2e ./e2e
COPY e2e-smoke ./e2e-smoke
COPY e2e-stack ./e2e-stack
COPY playwright.config.ts playwright.smoke.config.ts playwright.stack.config.ts vite.e2e.config.ts vitest.config.ts ./
EXPOSE 3000
CMD ["npm", "run", "dev"]
