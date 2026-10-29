import http from "k6/http";
import { check } from "k6";


const BASE_URLS = (__ENV.BASE_URLS || "http://app-dc1:8080,http://app-dc2:8080")
  .split(",")
  .map((url) => url.trim())
  .filter((url) => url.length > 0);

const TARGET_RPS = Number(__ENV.TARGET_RPS || 36);
const RAMP_MINUTES = Number(__ENV.RAMP_MINUTES || 3);
const HOLD_HOURS = Number(__ENV.HOLD_HOURS || 12);

// доли запросов
const CREATE = 0.3;
const GET = 0.65;
const UPDATE = 0.95;

const MISSING_ID = 99999999;
const MAX_ITEMS = 1000;

const CODES = [200, 404, 500];

export const options = {
  scenarios: {
    background_load: {
      executor: "ramping-arrival-rate",
      startRate: 2,
      timeUnit: "1s",
      preAllocatedVUs: 20,
      maxVUs: 100,
      stages: [
        { duration: `${RAMP_MINUTES}m`, target: TARGET_RPS },
        { duration: `${HOLD_HOURS}h`, target: TARGET_RPS },
      ],
    },
  },
};


const createdItems = [];

function baseUrl() {
  return BASE_URLS[Math.floor(Math.random() * BASE_URLS.length)];
}

function knownItem() {
  return createdItems[Math.floor(Math.random() * createdItems.length)];
}

function randomQuantity() {
  return Math.floor(Math.random() * 100);
}

function createItem(url) {
  const response = http.post(
    `${url}/items`,
    JSON.stringify({ name: `item-${Date.now()}`, quantity: randomQuantity() }),
    { headers: { "Content-Type": "application/json" } },
  );
  check(response, {
    "create answered": (r) => r.status === 201 || r.status === 500,
  });

  if (response.status === 201) {
    const item = JSON.parse(response.body);
    createdItems.push({ id: item.id, url });

    if (createdItems.length > MAX_ITEMS) {
      createdItems.shift();
    }
  }
}

function getItem(url, id) {
  const response = http.get(`${url}/items/${id}`);
  check(response, {
    "get answered": (r) => CODES.includes(r.status),
  });
}

function updateItem(url, id) {
  const response = http.put(
    `${url}/items/${id}`,
    JSON.stringify({ name: `item-${id}-updated`, quantity: randomQuantity() }),
    { headers: { "Content-Type": "application/json" } },
  );

  check(response, {
    "update answered": (r) => CODES.includes(r.status),
  });
}

export default function () {
  const url = baseUrl();

  if (createdItems.length === 0) {
    createItem(url);
    return;
  }

  const roll = Math.random();
  if (roll < CREATE) {
    createItem(url);
    return;
  }
  if (roll < GET) {
    const item = knownItem();
    getItem(item.url, item.id);
    return;
  }
  if (roll < UPDATE) {
    const item = knownItem();
    updateItem(item.url, item.id);
    return;
  }
   
  // для 404
  getItem(url, MISSING_ID);
}
