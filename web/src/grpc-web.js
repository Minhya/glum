import { createPromiseClient } from '@connectrpc/connect';
import { createGrpcWebTransport } from '@connectrpc/connect-web';
import { Timestamp } from '@bufbuild/protobuf';
import { Notes } from './gen/notes_connect.js';
import { Todos } from './gen/todo_connect.js';
import { Users } from './gen/users_connect.js';
import { Priority } from './gen/todo_pb.js';

const ENVOY_URL = window.location.origin;

function authInterceptor(token) {
  return (next) => async (request) => {
    request.header.set('Authorization', `Bearer ${token}`);
    return next(request);
  };
}

function clients(token) {
  const transport = createGrpcWebTransport({
    baseUrl: ENVOY_URL,
    interceptors: [authInterceptor(token)],
  });

  return {
    notes: createPromiseClient(Notes, transport),
    todos: createPromiseClient(Todos, transport),
    users: createPromiseClient(Users, transport),
  };
}

function dateToTimestamp(value) {
  if (!value) return undefined;

  const [datePart, timePart] = value.split('T');
  const date = new Date(timePart ? value : `${datePart}T12:00:00`);
  if (Number.isNaN(date.getTime())) return undefined;

  return Timestamp.fromDate(date);
}

export const api = {
  listNotes: (token) => clients(token).notes.listNotes({}),
  getNote: (token, id) => clients(token).notes.getNote({ id }),
  createNote: (token, title, content) => clients(token).notes.createNote({ title, content }),
  updateNote: (token, id, title, content) => clients(token).notes.updateNote({ id, title, content }),
  deleteNote: (token, id) => clients(token).notes.deleteNote({ id }),
  shareNote: (token, id, userId, canWrite) => clients(token).notes.shareNote({ id, userId, canWrite }),
  unshareNote: (token, id, userId) => clients(token).notes.unshareNote({ id, userId }),
  listTodos: (token) => clients(token).todos.listTodos({}),
  getTodo: (token, id) => clients(token).todos.getTodo({ id }),
  createTodo: (token, title, priority = Priority.LOW, dueDate, dueHasTime = false, syncToCalendar = false) => (
    clients(token).todos.createTodo({
      title,
      priority,
      dueDate: dateToTimestamp(dueDate),
      dueHasTime,
      syncToCalendar,
    })
  ),
  updateTodo: (token, id, title, priority = Priority.LOW, dueDate, dueHasTime = false) => (
    clients(token).todos.updateTodo({
      id,
      title,
      priority,
      dueDate: dateToTimestamp(dueDate),
      dueHasTime,
    })
  ),
  toggleTodo: (token, id) => clients(token).todos.toggleTodo({ id }),
  deleteTodo: (token, id) => clients(token).todos.deleteTodo({ id }),
  shareTodo: (token, id, userId, canWrite) => clients(token).todos.shareTodo({ id, userId, canWrite }),
  unshareTodo: (token, id, userId) => clients(token).todos.unshareTodo({ id, userId }),
  setTodoCalendarSync: (token, id, syncEnabled) => (
    clients(token).todos.setTodoCalendarSync({ id, syncEnabled })
  ),
  listUsers: (token, query = '', limit = 100) => clients(token).users.listUsers({ query, limit }),
};
