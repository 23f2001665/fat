import client from './client'

export async function getWorkoutDay(date) {
  const { data } = await client.get('/workouts', { params: { date } })
  return data
}

export async function getWorkoutHistory() {
  const { data } = await client.get('/workouts/history')
  return data.days
}

export async function createWorkoutSet(payload) {
  const { data } = await client.post('/workouts', payload)
  return data
}

export async function updateWorkoutSet(date, sequenceNumber, payload) {
  const { data } = await client.put(`/workouts/${date}/${sequenceNumber}`, payload)
  return data
}

export async function deleteWorkoutSet(date, sequenceNumber) {
  const { data } = await client.delete(`/workouts/${date}/${sequenceNumber}`)
  return data
}
