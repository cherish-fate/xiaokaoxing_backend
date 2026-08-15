<template>
  <div class="pagination">
    <button class="btn btn-ghost" type="button" :disabled="page <= 1" @click="change(page - 1)">
      <ChevronLeft :size="16" />
    </button>
    <span class="page-info">第 {{ page }} / {{ totalPages }} 页</span>
    <button class="btn btn-ghost" type="button" :disabled="page >= totalPages" @click="change(page + 1)">
      <ChevronRight :size="16" />
    </button>
    <span class="total-text">共 {{ total }} 条</span>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { ChevronLeft, ChevronRight } from '@lucide/vue'

const props = defineProps({
  page: { type: Number, default: 1 },
  total: { type: Number, default: 0 },
  pageSize: { type: Number, default: 20 }
})

const emit = defineEmits(['change'])

const totalPages = computed(() => Math.max(1, Math.ceil(props.total / props.pageSize)))

function change(page) {
  if (page < 1 || page > totalPages.value) return
  emit('change', page)
}
</script>