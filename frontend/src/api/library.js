import client from './client'

export async function getExercises({ includeInactive = false, search = '' } = {}) {
  const { data } = await client.get('/library/exercises', {
    params: { include_inactive: includeInactive, search: search || undefined },
  })
  return data.exercises
}

export async function createExercise(payload) {
  const { data } = await client.post('/library/exercises', payload)
  return data.exercise
}

export async function updateExercise(id, payload) {
  const { data } = await client.put(`/library/exercises/${id}`, payload)
  return data.exercise
}

export async function setExerciseActive(id, isActive) {
  const { data } = await client.patch(`/library/exercises/${id}/active`, {
    is_active: isActive,
  })
  return data.exercise
}

export async function getMuscleGroups({ includeInactive = false, search = '' } = {}) {
  const { data } = await client.get('/library/muscle-groups', {
    params: { include_inactive: includeInactive, search: search || undefined },
  })
  return data.muscle_groups
}

export async function createMuscleGroup(payload) {
  const { data } = await client.post('/library/muscle-groups', payload)
  return data.muscle_group
}

export async function updateMuscleGroup(id, payload) {
  const { data } = await client.put(`/library/muscle-groups/${id}`, payload)
  return data.muscle_group
}

export async function setMuscleGroupActive(id, isActive) {
  const { data } = await client.patch(`/library/muscle-groups/${id}/active`, {
    is_active: isActive,
  })
  return data.muscle_group
}

export async function getEquipment({ includeInactive = false, search = '' } = {}) {
  const { data } = await client.get('/library/equipment', {
    params: { include_inactive: includeInactive, search: search || undefined },
  })
  return data.equipment
}

export async function createEquipment(payload) {
  const { data } = await client.post('/library/equipment', payload)
  return data.equipment
}

export async function updateEquipment(id, payload) {
  const { data } = await client.put(`/library/equipment/${id}`, payload)
  return data.equipment
}

export async function setEquipmentActive(id, isActive) {
  const { data } = await client.patch(`/library/equipment/${id}/active`, {
    is_active: isActive,
  })
  return data.equipment
}
