import client from './client'

export async function getAnalytics({
  period = '30',
  start,
  end,
  exerciseId,
  muscleGroupId,
} = {}) {
  const { data } = await client.get('/analytics', {
    params: {
      period,
      start: start || undefined,
      end: end || undefined,
      exercise_id: exerciseId || undefined,
      muscle_group_id: muscleGroupId || undefined,
    },
  })
  return data
}
