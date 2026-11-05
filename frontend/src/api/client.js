import axios from 'axios'

const client = axios.create({
  baseURL: '/api',
  timeout: 20000,
  headers: {
    'Content-Type': 'application/json',
  },
})

client.interceptors.response.use(
  (response) => response,
  (error) => {
    const message =
      error.response?.data?.error ||
      (error.code === 'ECONNABORTED'
        ? 'The request timed out. Please try again.'
        : error.request
          ? 'The server could not be reached.'
          : error.message)
    error.userMessage = message
    return Promise.reject(error)
  },
)

export default client
