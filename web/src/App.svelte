<script>
  import { onMount } from 'svelte';
  import { login, logout, reLogin, handleCallback, getToken } from './auth.js';
  import { api } from './grpc-web.js';
  import { calendarActionLabel, calendarStatusClass, calendarStatusLabel, nextCalendarSyncState } from './lib/calendar.js';
  import { ensureShareDefaults, replaceById } from './lib/collections.js';
  import { dateInputValue, formatDue, formatTimestamp, timeInputValue, todoDueValue } from './lib/dates.js';
  import { accessLabel, canEditItem, canManageItem } from './lib/permissions.js';
  import { priorities, priorityClass, priorityLabel } from './lib/priorities.js';
  import { getInitialTheme, persistTheme, syncTheme } from './lib/theme.js';
  import { selectedUserLabel as selectedUserName, userDetail, userInitials, userMatches, userName } from './lib/users.js';
  import pandaArt from './assets/panda-japandi-art.png';

  let token = null;
  let notes = [];
  let todos = [];
  let users = [];
  let userSearch = '';
  let newNoteTitle = '';
  let newNoteContent = '';
  let newTodoTitle = '';
  let newTodoPriority = 1;
  let newTodoDueDate = '';
  let newTodoDueTime = '';
  let newTodoSyncCalendar = false;
  let error = null;
  let theme = getInitialTheme();

  $: isDarkTheme = theme === 'dark';
  $: syncTheme(theme);

  function setTheme(nextTheme) {
    theme = nextTheme === 'dark' ? 'dark' : 'light';
    persistTheme(theme);
  }

  function handleThemeChange(event) {
    setTheme(event.currentTarget.checked ? 'dark' : 'light');
  }

  $: completedTodos = todos.filter((todo) => todo.completed).length;
  $: activeTodos = todos.length - completedTodos;
  $: visibleUsers = users.filter((user) => userMatches(user, userSearch));

  async function run(action) {
    error = null;
    try {
      return await action();
    } catch (err) {
      error = err.message || 'Request failed';
      if (/invalid or expired token|invalid token/i.test(error)) {
        token = null;
        reLogin();
      }
      return null;
    }
  }

  function selectedUserLabel(userId) {
    return selectedUserName(users, userId);
  }

  async function loadUsers(query = '') {
    const data = await run(() => api.listUsers(token, query));
    if (!data) return;
    users = data.users || [];
  }

  function toggleNoteShare(note) {
    note.shareOpen = !note.shareOpen;
    note.shareNotice = '';
    ensureShareDefaults(note);
    notes = notes;
    if (note.shareOpen && users.length === 0) {
      loadUsers();
    }
  }

  function toggleTodoShare(todo) {
    todo.shareOpen = !todo.shareOpen;
    todo.shareNotice = '';
    ensureShareDefaults(todo);
    todos = todos;
    if (todo.shareOpen && users.length === 0) {
      loadUsers();
    }
  }

  function selectNoteShareUser(note, userId) {
    note.shareUserId = userId;
    ensureShareDefaults(note);
    note.shareNotice = '';
    notes = notes;
  }

  function selectTodoShareUser(todo, userId) {
    todo.shareUserId = userId;
    ensureShareDefaults(todo);
    todo.shareNotice = '';
    todos = todos;
  }

  async function loadNotes() {
    const data = await run(() => api.listNotes(token));
    if (!data) return;
    notes = data.notes || [];
  }

  async function loadTodos() {
    const data = await run(() => api.listTodos(token));
    if (!data) return;
    todos = data.todos || [];
  }

  async function createNote() {
    const title = newNoteTitle.trim();
    if (!title) return;

    const note = await run(() => api.createNote(token, title, newNoteContent));
    if (!note) return;

    newNoteTitle = '';
    newNoteContent = '';
    await loadNotes();
  }

  async function reloadNote(note) {
    const freshNote = await run(() => api.getNote(token, note.id));
    if (!freshNote) return;
    notes = replaceById(notes, freshNote);
  }

  function startNoteEdit(note) {
    if (!canEditItem(note)) return;
    note.editing = true;
    note.editTitle = note.title;
    note.editContent = note.content || '';
    notes = notes;
  }

  function cancelNoteEdit(note) {
    note.editing = false;
    note.editTitle = '';
    note.editContent = '';
    notes = notes;
  }

  async function saveNote(note) {
    if (!canEditItem(note)) return;
    const title = (note.editTitle || '').trim();
    if (!title) return;

    const updatedNote = await run(() => api.updateNote(token, note.id, title, note.editContent || ''));
    if (!updatedNote) return;

    notes = replaceById(notes, updatedNote);
  }

  async function deleteNote(id) {
    const response = await run(() => api.deleteNote(token, id));
    if (!response) return;
    await loadNotes();
  }

  async function shareNote(note) {
    const userId = (note.shareUserId || '').trim();
    if (!userId) return;

    const response = await run(() => api.shareNote(token, note.id, userId, note.shareCanWrite !== false));
    if (!response) return;

    note.shareNotice = `Shared with ${selectedUserLabel(userId)} as ${note.shareCanWrite === false ? 'view only' : 'can edit'}`;
    notes = notes;
  }

  async function unshareNote(note) {
    const userId = (note.shareUserId || '').trim();
    if (!userId) return;

    const response = await run(() => api.unshareNote(token, note.id, userId));
    if (!response) return;

    note.shareNotice = `Removed access for ${selectedUserLabel(userId)}`;
    note.shareCanWrite = true;
    notes = notes;
  }

  async function createTodo() {
    const title = newTodoTitle.trim();
    if (!title) return;

    const todo = await run(() => api.createTodo(
      token,
      title,
      Number(newTodoPriority),
      todoDueValue(newTodoDueDate, newTodoDueTime),
      Boolean(newTodoDueDate && newTodoDueTime),
      Boolean(newTodoDueDate && newTodoSyncCalendar),
    ));
    if (!todo) return;

    newTodoTitle = '';
    newTodoPriority = 1;
    newTodoDueDate = '';
    newTodoDueTime = '';
    newTodoSyncCalendar = false;
    await loadTodos();
  }

  async function reloadTodo(todo) {
    const freshTodo = await run(() => api.getTodo(token, todo.id));
    if (!freshTodo) return;
    todos = replaceById(todos, freshTodo);
  }

  function startTodoEdit(todo) {
    if (!canEditItem(todo)) return;
    todo.editing = true;
    todo.editTitle = todo.title;
    todo.editPriority = Number(todo.priority || 0);
    todo.editDueDate = dateInputValue(todo.dueDate);
    todo.editDueTime = todo.dueHasTime ? timeInputValue(todo.dueDate) : '';
    todos = todos;
  }

  function cancelTodoEdit(todo) {
    todo.editing = false;
    todo.editTitle = '';
    todo.editPriority = Number(todo.priority || 0);
    todo.editDueDate = '';
    todo.editDueTime = '';
    todos = todos;
  }

  async function saveTodo(todo) {
    if (!canEditItem(todo)) return;
    const title = (todo.editTitle || '').trim();
    if (!title) return;

    const updatedTodo = await run(() => api.updateTodo(
      token,
      todo.id,
      title,
      Number(todo.editPriority),
      todoDueValue(todo.editDueDate, todo.editDueTime),
      Boolean(todo.editDueDate && todo.editDueTime),
    ));
    if (!updatedTodo) return;

    todos = replaceById(todos, updatedTodo);
  }

  async function toggleTodo(todo) {
    if (!canEditItem(todo)) return;

    const updatedTodo = await run(() => api.toggleTodo(token, todo.id));
    if (!updatedTodo) return;
    todos = replaceById(todos, updatedTodo);
  }

  async function deleteTodo(id) {
    const response = await run(() => api.deleteTodo(token, id));
    if (!response) return;
    await loadTodos();
  }

  async function setTodoCalendarSync(todo, syncEnabled) {
    if (syncEnabled && !todo.dueDate) return;

    const updatedTodo = await run(() => api.setTodoCalendarSync(token, todo.id, syncEnabled));
    if (!updatedTodo) return;
    todos = replaceById(todos, updatedTodo);
  }

  async function shareTodo(todo) {
    const userId = (todo.shareUserId || '').trim();
    if (!userId) return;

    const response = await run(() => api.shareTodo(token, todo.id, userId, todo.shareCanWrite !== false));
    if (!response) return;

    todo.shareNotice = `Shared with ${selectedUserLabel(userId)} as ${todo.shareCanWrite === false ? 'view only' : 'can edit'}`;
    todos = todos;
  }

  async function unshareTodo(todo) {
    const userId = (todo.shareUserId || '').trim();
    if (!userId) return;

    const response = await run(() => api.unshareTodo(token, todo.id, userId));
    if (!response) return;

    todo.shareNotice = `Removed access for ${selectedUserLabel(userId)}`;
    todo.shareCanWrite = true;
    todos = todos;
  }

  onMount(async () => {
    await run(async () => {
      token = await handleCallback() || getToken();
      if (!token) return;

      await loadUsers();
      await loadNotes();
      await loadTodos();
    });
  });
</script>

{#if !token}
  <div class="login">
    <div class="login-backdrop" aria-hidden="true">
      <span></span>
      <span></span>
      <span></span>
    </div>
    <div class="login-shell">
      <section class="login-copy" aria-label="Glum">
        <div class="brand-lockup">
          <div class="brand-mark" aria-hidden="true">G</div>
          <div>
            <p class="eyebrow">Remember gently</p>
            <h1>Glum</h1>
            <p class="brand-copy">A quiet place for the things you almost forgot.</p>
          </div>
        </div>
        <figure class="panda-art" aria-hidden="true">
          <img src={pandaArt} alt="" />
        </figure>
      </section>
      <section class="login-panel" aria-label="Sign in">
        <div class="login-panel-top">
          <div class="brand-mark" aria-hidden="true">G</div>
          <label class="theme-slider" aria-label="Use dark theme">
            <input type="checkbox" checked={isDarkTheme} on:change={handleThemeChange} />
            <span>Light</span>
            <span>Dark</span>
            <i aria-hidden="true"></i>
          </label>
        </div>
        <p class="eyebrow">Offload gently</p>
        <h2>Let Glum hold the small things.</h2>
        <p class="login-panel-copy">Notes, todos, and shared thoughts stay close without crowding your head.</p>
        <button class="primary-action" on:click={login}>Continue with Google</button>
      </section>
    </div>
  </div>
{:else}
  <div class="app">
    <header class="app-header">
      <div class="brand-lockup">
        <div class="brand-mark" aria-hidden="true">G</div>
        <div>
          <p class="eyebrow">Workspace</p>
          <h1>Glum</h1>
        </div>
      </div>
      <div class="header-actions">
        <label class="theme-slider" aria-label="Use dark theme">
          <input type="checkbox" checked={isDarkTheme} on:change={handleThemeChange} />
          <span>Light</span>
          <span>Dark</span>
          <i aria-hidden="true"></i>
        </label>
        <span><strong>{notes.length}</strong> notes</span>
        <span><strong>{activeTodos}</strong> active</span>
        <span><strong>{completedTodos}</strong> done</span>
        <button class="ghost" on:click={logout}>Logout</button>
      </div>
    </header>

    {#if error}
      <p class="error">{error}</p>
    {/if}

    <main class="content-grid">
      <section class="panel">
        <div class="section-heading">
          <div>
            <p class="eyebrow">Capture</p>
            <h2>Notes</h2>
          </div>
          <div class="section-tools">
            <span class="count">{notes.length}</span>
            <button class="ghost small-button" type="button" on:click={loadNotes}>Refresh</button>
          </div>
        </div>

        <form class="form note-form" on:submit|preventDefault={createNote}>
          <label class="field">
            <span>Title</span>
            <input bind:value={newNoteTitle} placeholder="Note title" maxlength="100" />
          </label>
          <label class="field">
            <span>Content</span>
            <textarea bind:value={newNoteContent} placeholder="Note content" rows="3" maxlength="10000"></textarea>
          </label>
          <button class="primary-action" type="submit" disabled={!newNoteTitle.trim()}>Add Note</button>
        </form>

        <div class="list notes-list">
          {#if notes.length === 0}
            <p class="empty-state">No notes yet.</p>
          {:else}
            {#each notes as note}
              <article class="item note-card">
                {#if note.editing}
                  <form class="edit-form" on:submit|preventDefault={() => saveNote(note)}>
                    <label class="field">
                      <span>Title</span>
                      <input bind:value={note.editTitle} maxlength="100" />
                    </label>
                    <label class="field">
                      <span>Content</span>
                      <textarea bind:value={note.editContent} rows="4" maxlength="10000"></textarea>
                    </label>
                    <div class="item-actions">
                      <button class="primary-action" type="submit" disabled={!note.editTitle?.trim()}>Save</button>
                      <button class="ghost" type="button" on:click={() => cancelNoteEdit(note)}>Cancel</button>
                    </div>
                  </form>
                {:else}
                  <div class="item-top">
                    <div class="item-content">
                      <strong>{note.title}</strong>
                      {#if note.content}
                        <p>{note.content}</p>
                      {/if}
                      <div class="meta-line">
                        <span class:readonly-access={!canEditItem(note)}>{accessLabel(note)}</span>
                        {#if note.createdAt}
                          <span>Created {formatTimestamp(note.createdAt)}</span>
                        {/if}
                        {#if note.updatedAt}
                          <span>Updated {formatTimestamp(note.updatedAt)}</span>
                        {/if}
                      </div>
                    </div>
                    <div class="item-actions">
                      <button class="ghost small-button" type="button" on:click={() => reloadNote(note)}>Reload</button>
                      <button class="ghost small-button" type="button" disabled={!canEditItem(note)} title={!canEditItem(note) ? 'View-only shares cannot be edited' : undefined} on:click={() => startNoteEdit(note)}>Edit</button>
                      <button class="ghost small-button" type="button" disabled={!canManageItem(note)} title={!canManageItem(note) ? 'Only the owner can manage access' : undefined} on:click={() => toggleNoteShare(note)}>
                        {note.shareOpen ? 'Close share' : 'Share'}
                      </button>
                      <button class="ghost danger small-button" type="button" disabled={!canManageItem(note)} title={!canManageItem(note) ? 'Only the owner can delete this note' : undefined} on:click={() => deleteNote(note.id)}>Delete</button>
                    </div>
                  </div>
                {/if}

                {#if note.shareOpen}
                  <form class="share-panel" on:submit|preventDefault={() => shareNote(note)}>
                    <div class="share-panel-header">
                      <div>
                        <span class="share-kicker">Access</span>
                        <strong>Share note</strong>
                      </div>
                      <button class="ghost small-button" type="button" on:click={() => loadUsers(userSearch)}>Refresh users</button>
                    </div>

                    <label class="field">
                      <span>Find user</span>
                      <input bind:value={userSearch} placeholder="Search name or email" />
                    </label>

                    <div class="share-user-list" aria-label="Users">
                      {#if visibleUsers.length === 0}
                        <p class="empty-state compact-empty">No users match this search.</p>
                      {:else}
                        {#each visibleUsers as user}
                          <button
                            class:selected-user={note.shareUserId === user.id}
                            class="user-option"
                            type="button"
                            on:click={() => selectNoteShareUser(note, user.id)}
                          >
                            <span class="user-avatar" aria-hidden="true">{userInitials(user)}</span>
                            <span class="user-copy">
                              <strong>{userName(user)}</strong>
                              <small>{userDetail(user)}</small>
                            </span>
                          </button>
                        {/each}
                      {/if}
                    </div>

                    <div class="share-footer">
                      <div class="access-toggle">
                        <label class="switch-field">
                          <input type="checkbox" bind:checked={note.shareCanWrite} />
                          <span>Can edit</span>
                        </label>
                        <span class:readonly-access={note.shareCanWrite === false} class="permission-help">
                          {note.shareCanWrite === false ? 'View only' : 'Default access'}
                        </span>
                      </div>
                      <div class="access-actions">
                        <button class="primary-action small-button" type="submit" disabled={!note.shareUserId?.trim()}>Share</button>
                        <button class="ghost danger small-button" type="button" disabled={!note.shareUserId?.trim()} on:click={() => unshareNote(note)}>Remove access</button>
                      </div>
                    </div>

                    {#if note.shareNotice}
                      <p class="share-notice">{note.shareNotice}</p>
                    {/if}
                  </form>
                {/if}
              </article>
            {/each}
          {/if}
        </div>
      </section>

      <section class="panel">
        <div class="section-heading">
          <div>
            <p class="eyebrow">Track</p>
            <h2>Todos</h2>
          </div>
          <div class="section-tools">
            <span class="count">{todos.length}</span>
            <button class="ghost small-button" type="button" on:click={loadTodos}>Refresh</button>
          </div>
        </div>

        <form class="form todo-form" on:submit|preventDefault={createTodo}>
          <label class="field todo-title-field">
            <span>Title</span>
            <input bind:value={newTodoTitle} placeholder="Todo title" maxlength="100" />
          </label>
          <label class="field">
            <span>Priority</span>
            <select bind:value={newTodoPriority}>
              {#each priorities as priority}
                <option value={priority.value}>{priority.label}</option>
              {/each}
            </select>
          </label>
          <label class="field">
            <span>Due</span>
            <input type="date" bind:value={newTodoDueDate} />
          </label>
          <label class="field">
            <span>Time optional</span>
            <input type="time" bind:value={newTodoDueTime} disabled={!newTodoDueDate} />
          </label>
          <div class="form-action">
            <label class="switch-field calendar-create-toggle" title={!newTodoDueDate ? 'Choose a due date before syncing to Google Calendar' : undefined}>
              <input type="checkbox" bind:checked={newTodoSyncCalendar} disabled={!newTodoDueDate} />
              <span>Sync to Google Calendar</span>
            </label>
            <button class="primary-action" type="submit" disabled={!newTodoTitle.trim()}>Add Todo</button>
          </div>
        </form>

        <div class="list">
          {#if todos.length === 0}
            <p class="empty-state">No todos yet.</p>
          {:else}
            {#each todos as todo}
              <div class="item todo-card" class:completed={todo.completed}>
                {#if todo.editing}
                  <form class="edit-form" on:submit|preventDefault={() => saveTodo(todo)}>
                    <div class="todo-edit-grid">
                      <label class="field todo-title-field">
                        <span>Title</span>
                        <input bind:value={todo.editTitle} maxlength="100" />
                      </label>
                      <label class="field">
                        <span>Priority</span>
                        <select bind:value={todo.editPriority}>
                          {#each priorities as priority}
                            <option value={priority.value}>{priority.label}</option>
                          {/each}
                        </select>
                      </label>
                      <label class="field">
                        <span>Due</span>
                        <input type="date" bind:value={todo.editDueDate} />
                      </label>
                      <label class="field">
                        <span>Time optional</span>
                        <input type="time" bind:value={todo.editDueTime} disabled={!todo.editDueDate} />
                      </label>
                    </div>
                    <div class="item-actions">
                      <button class="primary-action" type="submit" disabled={!todo.editTitle?.trim()}>Save</button>
                      <button class="ghost" type="button" on:click={() => cancelTodoEdit(todo)}>Cancel</button>
                    </div>
                  </form>
                {:else}
                  <div class="item-top">
                    <div class="item-content">
                      <label class="todo-title">
                        <input
                          type="checkbox"
                          checked={todo.completed}
                          disabled={!canEditItem(todo)}
                          title={!canEditItem(todo) ? 'View-only shares cannot be changed' : undefined}
                          aria-label={todo.completed ? `Mark ${todo.title} active` : `Mark ${todo.title} done`}
                          on:change={() => toggleTodo(todo)}
                        />
                        <strong>{todo.title}</strong>
                      </label>
                      <div class="meta-line">
                        <span class:readonly-access={!canEditItem(todo)}>{accessLabel(todo)}</span>
                        <span class={`priority priority-${priorityClass(todo.priority)}`}>{priorityLabel(todo.priority)}</span>
                        {#if todo.dueDate}
                          <span>Due {formatDue(todo)}</span>
                        {/if}
                        <span class={calendarStatusClass(todo)} title={todo.calendarSyncError || undefined}>
                          {calendarStatusLabel(todo)}
                        </span>
                        {#if todo.createdAt}
                          <span>Created {formatTimestamp(todo.createdAt)}</span>
                        {/if}
                        {#if todo.updatedAt}
                          <span>Updated {formatTimestamp(todo.updatedAt)}</span>
                        {/if}
                      </div>
                    </div>
                    <div class="item-actions">
                      <button class="ghost small-button" type="button" on:click={() => reloadTodo(todo)}>Reload</button>
                      <button class="ghost small-button" type="button" disabled={!canEditItem(todo)} title={!canEditItem(todo) ? 'View-only shares cannot be edited' : undefined} on:click={() => startTodoEdit(todo)}>Edit</button>
                      <button
                        class="ghost small-button calendar-button"
                        type="button"
                        disabled={!todo.dueDate}
                        title={!todo.dueDate ? 'Choose a due date before syncing to Google Calendar' : undefined}
                        on:click={() => setTodoCalendarSync(todo, nextCalendarSyncState(todo))}
                      >
                        {calendarActionLabel(todo)}
                      </button>
                      <button class="ghost small-button" type="button" disabled={!canManageItem(todo)} title={!canManageItem(todo) ? 'Only the owner can manage access' : undefined} on:click={() => toggleTodoShare(todo)}>
                        {todo.shareOpen ? 'Close share' : 'Share'}
                      </button>
                      <button class="ghost danger small-button" type="button" disabled={!canManageItem(todo)} title={!canManageItem(todo) ? 'Only the owner can delete this todo' : undefined} on:click={() => deleteTodo(todo.id)}>Delete</button>
                    </div>
                  </div>
                {/if}

                {#if todo.shareOpen}
                  <form class="share-panel" on:submit|preventDefault={() => shareTodo(todo)}>
                    <div class="share-panel-header">
                      <div>
                        <span class="share-kicker">Access</span>
                        <strong>Share todo</strong>
                      </div>
                      <button class="ghost small-button" type="button" on:click={() => loadUsers(userSearch)}>Refresh users</button>
                    </div>

                    <label class="field">
                      <span>Find user</span>
                      <input bind:value={userSearch} placeholder="Search name or email" />
                    </label>

                    <div class="share-user-list" aria-label="Users">
                      {#if visibleUsers.length === 0}
                        <p class="empty-state compact-empty">No users match this search.</p>
                      {:else}
                        {#each visibleUsers as user}
                          <button
                            class:selected-user={todo.shareUserId === user.id}
                            class="user-option"
                            type="button"
                            on:click={() => selectTodoShareUser(todo, user.id)}
                          >
                            <span class="user-avatar" aria-hidden="true">{userInitials(user)}</span>
                            <span class="user-copy">
                              <strong>{userName(user)}</strong>
                              <small>{userDetail(user)}</small>
                            </span>
                          </button>
                        {/each}
                      {/if}
                    </div>

                    <div class="share-footer">
                      <div class="access-toggle">
                        <label class="switch-field">
                          <input type="checkbox" bind:checked={todo.shareCanWrite} />
                          <span>Can edit</span>
                        </label>
                        <span class:readonly-access={todo.shareCanWrite === false} class="permission-help">
                          {todo.shareCanWrite === false ? 'View only' : 'Default access'}
                        </span>
                      </div>
                      <div class="access-actions">
                        <button class="primary-action small-button" type="submit" disabled={!todo.shareUserId?.trim()}>Share</button>
                        <button class="ghost danger small-button" type="button" disabled={!todo.shareUserId?.trim()} on:click={() => unshareTodo(todo)}>Remove access</button>
                      </div>
                    </div>

                    {#if todo.shareNotice}
                      <p class="share-notice">{todo.shareNotice}</p>
                    {/if}
                  </form>
                {/if}
              </div>
            {/each}
          {/if}
        </div>
      </section>
    </main>
  </div>
{/if}

<style>
  :global(:root) {
    --bg: #f4f7f8;
    --surface: #ffffff;
    --surface-soft: #f8faf8;
    --surface-warm: #fbfaf6;
    --ink: #18202b;
    --muted: #657286;
    --faint: #8793a3;
    --line: #d9e1e8;
    --line-strong: #c7d1dc;
    --blue: #2f6fed;
    --blue-dark: #245bc4;
    --green: #17795f;
    --green-soft: #eaf6f1;
    --amber: #a05a00;
    --amber-soft: #fff5db;
    --red: #b42318;
    --red-soft: #fff2f0;
    --shadow: 0 14px 30px rgb(28 36 48 / 8%);
  }

  :global(*) {
    box-sizing: border-box;
  }

  :global(html) {
    min-width: 320px;
    background: var(--bg);
  }

  :global(body) {
    min-width: 320px;
    min-height: 100vh;
    margin: 0;
    background:
      linear-gradient(180deg, #eaf0f3 0, #f4f7f8 320px, #f6f7f4 100%);
    color: var(--ink);
    font-family:
      Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI",
      sans-serif;
    letter-spacing: 0;
    text-rendering: optimizeLegibility;
  }

  :global(button),
  :global(input),
  :global(select),
  :global(textarea) {
    font: inherit;
  }

  :global(button),
  :global(input),
  :global(select),
  :global(textarea) {
    color: inherit;
  }

  .app,
  .login {
    width: min(1180px, calc(100vw - 32px));
    margin: 0 auto;
  }

  .app {
    padding: 28px 0 56px;
  }

  .login {
    display: grid;
    min-height: 100vh;
    place-items: center;
    padding: 48px 0;
    width: min(960px, calc(100vw - 32px));
  }

  .login-shell {
    display: grid;
    width: 100%;
    grid-template-columns: minmax(0, 1fr) minmax(300px, 340px);
    gap: 18px;
    align-items: center;
  }

  .login-copy,
  .login-panel {
    border: 1px solid rgb(255 255 255 / 78%);
    border-radius: 8px;
    background: rgb(255 255 255 / 86%);
    box-shadow: var(--shadow);
    backdrop-filter: blur(10px);
  }

  .login-copy {
    display: grid;
    min-height: 360px;
    align-content: space-between;
    gap: 24px;
    overflow: hidden;
    padding: 28px;
    background:
      linear-gradient(135deg, rgb(255 255 255 / 92%), rgb(238 246 242 / 92%));
  }

  .login-panel {
    align-self: center;
    display: grid;
    gap: 14px;
    padding: 28px;
    text-align: left;
  }

  .login-panel h2 {
    font-size: 2rem;
  }

  .login-panel .primary-action {
    width: 100%;
    margin-top: 4px;
  }

  .app-header,
  .brand-lockup,
  .section-heading,
  .header-actions,
  .section-tools,
  .item-top,
  .item-actions,
  .meta-line,
  .share-panel-header,
  .share-footer,
  .access-toggle,
  .access-actions,
  .switch-field,
  .user-option,
  .todo-title {
    display: flex;
    align-items: center;
  }

  .app-header {
    justify-content: space-between;
    gap: 20px;
    margin-bottom: 28px;
    padding: 18px;
    border: 1px solid rgb(255 255 255 / 78%);
    border-radius: 8px;
    background: rgb(255 255 255 / 82%);
    box-shadow: var(--shadow);
    backdrop-filter: blur(10px);
  }

  .brand-lockup {
    min-width: 0;
    gap: 14px;
  }

  .brand-mark {
    display: inline-grid;
    width: 44px;
    height: 44px;
    flex: 0 0 auto;
    place-items: center;
    border: 1px solid #c9d8d3;
    border-radius: 8px;
    background: #12372f;
    color: #f8fbf9;
    font-weight: 900;
    line-height: 1;
  }

  .login-panel .brand-mark {
    width: 54px;
    height: 54px;
    margin: 0 auto 18px;
    font-size: 1.2rem;
  }

  .header-actions {
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 8px;
  }

  .header-actions span,
  .count {
    display: inline-flex;
    min-height: 34px;
    align-items: center;
    gap: 5px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--surface-soft);
    color: var(--muted);
    font-size: 0.86rem;
    font-weight: 750;
    padding: 6px 10px;
  }

  .header-actions strong {
    color: var(--ink);
    font-size: 0.98rem;
  }

  h1,
  h2,
  p {
    margin: 0;
  }

  h1 {
    color: #111820;
    font-size: clamp(2rem, 4vw, 2.8rem);
    line-height: 0.96;
    letter-spacing: 0;
  }

  h2 {
    color: #111820;
    font-size: 1.35rem;
    line-height: 1.2;
  }

  .eyebrow {
    margin-bottom: 6px;
    color: var(--faint);
    font-size: 0.75rem;
    font-weight: 800;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .content-grid {
    display: grid;
    grid-template-columns: minmax(0, 1.15fr) minmax(360px, 0.85fr);
    gap: 28px;
    align-items: start;
  }

  .panel {
    min-width: 0;
  }

  .section-heading {
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 14px;
  }

  .section-tools {
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 8px;
  }

  .form {
    display: grid;
    gap: 12px;
    margin-bottom: 14px;
    padding: 14px;
    border: 1px solid rgb(255 255 255 / 82%);
    border-radius: 8px;
    background: rgb(255 255 255 / 88%);
    box-shadow: 0 10px 24px rgb(28 36 48 / 6%);
  }

  .note-form {
    background:
      linear-gradient(180deg, rgb(255 255 255 / 92%), rgb(248 250 248 / 92%));
  }

  .todo-form {
    grid-template-columns: minmax(170px, 1fr) minmax(115px, 0.28fr) minmax(145px, 0.34fr) minmax(120px, 0.28fr);
    align-items: start;
  }

  .form-action {
    grid-column: 1 / -1;
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    justify-content: flex-end;
    padding-top: 2px;
  }

  .calendar-create-toggle {
    margin-right: auto;
  }

  .form-action .primary-action {
    min-width: 132px;
  }

  .field {
    display: grid;
    min-width: 0;
    gap: 6px;
  }

  .field > span {
    color: var(--muted);
    font-size: 0.76rem;
    font-weight: 800;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  input,
  select,
  textarea {
    width: 100%;
    min-height: 44px;
    border: 1px solid var(--line-strong);
    border-radius: 6px;
    background: #ffffff;
    color: var(--ink);
    padding: 10px 11px;
    outline: none;
    transition:
      background-color 120ms ease,
      border-color 120ms ease,
      box-shadow 120ms ease;
  }

  select {
    cursor: pointer;
  }

  textarea {
    min-height: 92px;
    resize: vertical;
  }

  input:focus,
  select:focus,
  textarea:focus {
    border-color: var(--blue);
    box-shadow: 0 0 0 3px rgb(47 111 237 / 16%);
  }

  input::placeholder,
  textarea::placeholder {
    color: #9aa5b3;
  }

  button {
    min-height: 42px;
    border: 0;
    border-radius: 6px;
    cursor: pointer;
    font-weight: 850;
    padding: 0 14px;
    transition:
      background-color 120ms ease,
      color 120ms ease,
      border-color 120ms ease,
      opacity 120ms ease,
      transform 120ms ease,
      box-shadow 120ms ease;
    white-space: nowrap;
  }

  button:not(:disabled):active {
    transform: translateY(1px);
  }

  button:focus-visible {
    outline: 3px solid rgb(47 111 237 / 22%);
    outline-offset: 2px;
  }

  button:disabled {
    cursor: not-allowed;
    opacity: 0.45;
  }

  .primary-action {
    background: var(--blue);
    color: #ffffff;
    box-shadow: 0 8px 16px rgb(47 111 237 / 18%);
  }

  .primary-action:not(:disabled):hover {
    background: var(--blue-dark);
  }

  .ghost {
    border: 1px solid var(--line-strong);
    background: rgb(255 255 255 / 88%);
    color: #39465a;
  }

  .ghost:not(:disabled):hover {
    border-color: #9dacbc;
    background: #f5f8f9;
  }

  .small-button {
    min-height: 34px;
    padding: 0 10px;
    font-size: 0.84rem;
  }

  .danger {
    color: var(--red);
  }

  .danger:not(:disabled):hover {
    border-color: #f0aaa3;
    background: var(--red-soft);
  }

  .error,
  .empty-state {
    border-radius: 8px;
    padding: 13px 14px;
  }

  .error {
    margin-bottom: 18px;
    border: 1px solid #f0aaa3;
    background: var(--red-soft);
    color: #9f1c13;
    font-weight: 700;
  }

  .empty-state {
    border: 1px dashed var(--line-strong);
    background: rgb(255 255 255 / 68%);
    color: var(--muted);
    font-weight: 650;
  }

  .list {
    display: grid;
    gap: 12px;
  }

  .item {
    display: grid;
    min-width: 0;
    gap: 14px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: rgb(255 255 255 / 92%);
    box-shadow: 0 10px 24px rgb(28 36 48 / 5%);
    padding: 14px;
    transition:
      border-color 120ms ease,
      box-shadow 120ms ease,
      transform 120ms ease;
  }

  .item:hover {
    border-color: #c5d0da;
    box-shadow: 0 14px 26px rgb(28 36 48 / 8%);
    transform: translateY(-1px);
  }

  .item-top {
    min-width: 0;
    justify-content: space-between;
    gap: 14px;
  }

  .item-content {
    min-width: 0;
  }

  .item strong {
    color: var(--ink);
    font-size: 1rem;
    line-height: 1.35;
  }

  .item strong,
  .item p,
  .meta-line span {
    overflow-wrap: anywhere;
  }

  .note-card {
    align-items: flex-start;
  }

  .note-card div {
    min-width: 0;
  }

  .note-card p {
    margin-top: 6px;
    color: #4f5e70;
    line-height: 1.5;
    white-space: pre-wrap;
  }

  .item-actions {
    flex: 0 0 auto;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 8px;
  }

  .edit-form {
    display: grid;
    width: 100%;
    gap: 12px;
  }

  .todo-edit-grid {
    display: grid;
    grid-template-columns: minmax(170px, 1fr) minmax(115px, 0.28fr) minmax(145px, 0.34fr) minmax(120px, 0.28fr);
    gap: 12px;
  }

  .todo-title {
    min-width: 0;
    gap: 10px;
    color: var(--ink);
    cursor: pointer;
  }

  .todo-title input {
    width: 20px;
    height: 20px;
    min-height: 20px;
    flex: 0 0 20px;
    accent-color: var(--green);
    cursor: pointer;
  }

  .completed .todo-title strong {
    color: var(--faint);
    text-decoration: line-through;
  }

  .completed {
    background: rgb(255 255 255 / 72%);
  }

  .meta-line {
    flex-wrap: wrap;
    gap: 7px;
    margin-top: 9px;
    color: var(--muted);
    font-size: 0.82rem;
    line-height: 1.35;
  }

  .meta-line span {
    display: inline-flex;
    min-height: 26px;
    align-items: center;
    border: 1px solid #dde4eb;
    border-radius: 6px;
    background: #f8fafb;
    padding: 4px 8px;
  }

  .priority {
    font-weight: 850;
  }

  .priority-low {
    border-color: #cbe7db;
    background: var(--green-soft);
    color: var(--green);
  }

  .priority-medium {
    border-color: #f1d694;
    background: var(--amber-soft);
    color: var(--amber);
  }

  .priority-high {
    border-color: #f0aaa3;
    background: var(--red-soft);
    color: var(--red);
  }

  .readonly-access {
    border-color: #f0d6a3 !important;
    background: #fff8e7 !important;
    color: #8a5200 !important;
    font-weight: 850;
  }

  .share-panel {
    display: grid;
    width: 100%;
    gap: 12px;
    border: 1px solid #d7e4ef;
    border-radius: 8px;
    background:
      linear-gradient(180deg, #fbfdff 0%, #f5f9fb 100%);
    box-shadow: inset 0 1px 0 rgb(255 255 255 / 78%);
    padding: 14px;
  }

  .share-panel-header,
  .share-footer {
    justify-content: space-between;
    gap: 12px;
  }

  .share-panel-header strong {
    display: block;
    font-size: 0.98rem;
  }

  .share-kicker {
    display: block;
    margin-bottom: 3px;
    color: var(--faint);
    font-size: 0.7rem;
    font-weight: 850;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .share-user-list {
    display: grid;
    max-height: 244px;
    gap: 8px;
    overflow-y: auto;
    padding-right: 2px;
  }

  .user-option {
    width: 100%;
    min-height: 58px;
    justify-content: flex-start;
    gap: 11px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: #ffffff;
    padding: 9px;
    text-align: left;
    white-space: normal;
  }

  .user-option:not(:disabled):hover,
  .selected-user {
    border-color: #8fb2ec;
    background: #f0f6ff;
    box-shadow: 0 6px 14px rgb(47 111 237 / 10%);
  }

  .user-avatar {
    display: inline-grid;
    width: 38px;
    height: 38px;
    flex: 0 0 38px;
    place-items: center;
    border-radius: 8px;
    background: #dfece7;
    color: #12372f;
    font-size: 0.8rem;
    font-weight: 900;
  }

  .user-copy {
    display: grid;
    min-width: 0;
    gap: 2px;
  }

  .user-copy strong,
  .user-copy small {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .user-copy strong {
    font-size: 0.92rem;
    line-height: 1.2;
  }

  .user-copy small {
    color: var(--muted);
    font-size: 0.76rem;
    font-weight: 650;
  }

  .access-toggle {
    flex-wrap: wrap;
    gap: 8px;
  }

  .switch-field {
    min-height: 34px;
    gap: 8px;
    border: 1px solid #d8e5ef;
    border-radius: 6px;
    background: #ffffff;
    color: #354258;
    font-size: 0.84rem;
    font-weight: 850;
    padding: 6px 10px;
  }

  .switch-field input {
    width: 17px;
    height: 17px;
    min-height: 17px;
    accent-color: var(--blue);
    cursor: pointer;
  }

  .permission-help {
    display: inline-flex;
    min-height: 34px;
    align-items: center;
    border: 1px solid #d8e5ef;
    border-radius: 6px;
    background: #f8fafb;
    color: var(--muted);
    font-size: 0.82rem;
    font-weight: 800;
    padding: 7px 10px;
  }

  .access-actions {
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 8px;
    margin-left: auto;
  }

  .share-notice {
    border: 1px solid #cbe7db;
    border-radius: 6px;
    background: var(--green-soft);
    color: var(--green);
    font-size: 0.84rem;
    font-weight: 750;
    padding: 8px 10px;
  }

  .compact-empty {
    padding: 10px;
  }

  @media (max-width: 820px) {
    .app,
    .login {
      width: min(100% - 20px, 1120px);
    }

    .app {
      padding-top: 20px;
    }

    .login-shell {
      grid-template-columns: 1fr;
    }

    .login-copy {
      min-height: auto;
    }

    .app-header,
    .content-grid {
      grid-template-columns: 1fr;
    }

    .app-header {
      align-items: flex-start;
      flex-direction: column;
    }

    .header-actions {
      justify-content: flex-start;
    }

    .content-grid {
      display: grid;
      gap: 16px;
    }

    .todo-form,
    .todo-edit-grid {
      grid-template-columns: 1fr;
    }

    .todo-form button,
    .note-form button,
    .form-action {
      width: 100%;
    }

    .form-action .primary-action {
      width: 100%;
    }
  }

  :global(:root) {
    color-scheme: dark;
    --bg: #070b0c;
    --surface: #111818;
    --surface-soft: #151f1f;
    --surface-warm: #1a1712;
    --ink: #eef6f2;
    --muted: #9caeaa;
    --faint: #6f837f;
    --line: #263636;
    --line-strong: #36504e;
    --blue: #4e8cff;
    --blue-dark: #77a7ff;
    --green: #64d6ad;
    --green-soft: #112a23;
    --amber: #ffbe58;
    --amber-soft: #2a2111;
    --red: #ff6f61;
    --red-soft: #321817;
    --shadow: 0 24px 60px rgb(0 0 0 / 36%);
  }

  :global(html) {
    background: var(--bg);
  }

  :global(body) {
    background:
      linear-gradient(115deg, rgb(100 214 173 / 6%) 0 1px, transparent 1px 44px),
      linear-gradient(0deg, #06090a 0%, #0b1111 48%, #0f1411 100%);
    color: var(--ink);
  }

  .login {
    position: relative;
    width: min(1040px, calc(100vw - 32px));
    overflow: hidden;
  }

  .login-backdrop {
    position: absolute;
    inset: 4vh -8vw;
    pointer-events: none;
    opacity: 0.9;
  }

  .login-backdrop::before {
    position: absolute;
    inset: 0;
    content: "";
    background:
      repeating-linear-gradient(90deg, rgb(100 214 173 / 12%) 0 1px, transparent 1px 88px),
      repeating-linear-gradient(0deg, rgb(78 140 255 / 9%) 0 1px, transparent 1px 72px);
    mask-image: linear-gradient(90deg, transparent, #000 20%, #000 80%, transparent);
    transform: perspective(700px) rotateX(56deg) translateY(70px);
    transform-origin: bottom;
    animation: grid-drift 14s linear infinite;
  }

  .login-backdrop span {
    position: absolute;
    left: 7%;
    right: 7%;
    height: 1px;
    background: linear-gradient(90deg, transparent, rgb(100 214 173 / 52%), transparent);
    transform: translateX(-18%);
    animation: signal-sweep 5.8s ease-in-out infinite;
  }

  .login-backdrop span:nth-child(1) {
    top: 18%;
  }

  .login-backdrop span:nth-child(2) {
    top: 48%;
    animation-delay: 1.2s;
  }

  .login-backdrop span:nth-child(3) {
    top: 72%;
    animation-delay: 2.4s;
  }

  .login-shell {
    position: relative;
    z-index: 1;
    gap: 20px;
  }

  .login-copy,
  .login-panel,
  .app-header,
  .form,
  .item {
    border-color: rgb(255 255 255 / 10%);
    background: linear-gradient(180deg, rgb(20 30 30 / 88%), rgb(12 18 18 / 86%));
    box-shadow: var(--shadow);
  }

  .login-copy {
    position: relative;
    min-height: 440px;
    background:
      linear-gradient(135deg, rgb(22 40 36 / 94%), rgb(11 15 16 / 92%) 58%, rgb(31 25 15 / 78%));
  }

  .login-copy::after {
    position: absolute;
    inset: 0;
    content: "";
    background:
      linear-gradient(90deg, transparent 0 32%, rgb(100 214 173 / 8%) 32% 33%, transparent 33%),
      linear-gradient(0deg, transparent 0 62%, rgb(255 190 88 / 8%) 62% 63%, transparent 63%);
    pointer-events: none;
  }

  .login-panel {
    background:
      linear-gradient(180deg, rgb(22 29 31 / 94%), rgb(11 15 16 / 94%));
    transform: translateY(0);
    animation: panel-arrive 520ms cubic-bezier(0.2, 0.8, 0.2, 1) both;
  }

  .brand-mark {
    border-color: rgb(100 214 173 / 32%);
    background: linear-gradient(135deg, #143b34, #09100f);
    color: #ecfff9;
    box-shadow: 0 0 0 1px rgb(255 255 255 / 5%), 0 12px 28px rgb(100 214 173 / 12%);
  }

  .login-panel .brand-mark {
    animation: mark-float 3.8s ease-in-out infinite;
  }

  h1,
  h2,
  .header-actions strong,
  .item strong {
    color: var(--ink);
  }

  .eyebrow {
    color: var(--green);
  }

  .header-actions span,
  .count,
  .meta-line span,
  .permission-help {
    border-color: var(--line);
    background: #101818;
    color: var(--muted);
  }

  .note-form,
  .todo-form,
  .share-panel {
    border-color: rgb(255 255 255 / 9%);
    background: linear-gradient(180deg, rgb(16 24 25 / 96%), rgb(10 15 16 / 94%));
  }

  input,
  select,
  textarea {
    border-color: var(--line-strong);
    background: #090f10;
    color: var(--ink);
  }

  input:focus,
  select:focus,
  textarea:focus {
    border-color: var(--green);
    box-shadow: 0 0 0 3px rgb(100 214 173 / 16%);
  }

  input::placeholder,
  textarea::placeholder {
    color: #657874;
  }

  .primary-action {
    background: linear-gradient(135deg, #2f6fed, #31b98f);
    color: #ffffff;
    box-shadow: 0 12px 22px rgb(49 185 143 / 18%);
  }

  .primary-action:not(:disabled):hover {
    background: linear-gradient(135deg, #77a7ff, #64d6ad);
    box-shadow: 0 14px 26px rgb(100 214 173 / 24%);
    transform: translateY(-1px);
  }

  .ghost {
    border-color: var(--line-strong);
    background: #0e1516;
    color: #c3d1ce;
  }

  .ghost:not(:disabled):hover {
    border-color: rgb(100 214 173 / 45%);
    background: #152120;
  }

  .item {
    background: #0d1415;
  }

  .item:hover {
    border-color: rgb(100 214 173 / 28%);
    box-shadow: 0 18px 34px rgb(0 0 0 / 34%);
  }

  .note-card p {
    color: #adbfba;
  }

  .completed {
    background: #0b1112;
  }

  .priority-low {
    border-color: rgb(100 214 173 / 34%);
    background: #112a23;
    color: var(--green);
  }

  .priority-medium {
    border-color: rgb(255 190 88 / 38%);
    background: #2a2111;
    color: var(--amber);
  }

  .priority-high,
  .danger:not(:disabled):hover,
  .error {
    border-color: rgb(255 111 97 / 38%);
    background: #321817;
    color: var(--red);
  }

  .readonly-access {
    border-color: rgb(255 190 88 / 38%) !important;
    background: #2a2111 !important;
    color: var(--amber) !important;
  }

  .user-option,
  .switch-field {
    border-color: var(--line);
    background: #0c1314;
    color: var(--ink);
  }

  .user-option:not(:disabled):hover,
  .selected-user {
    border-color: rgb(78 140 255 / 54%);
    background: #111d26;
    box-shadow: 0 10px 20px rgb(78 140 255 / 12%);
  }

  .user-avatar {
    background: #19352d;
    color: #bffbe7;
  }

  .user-copy small {
    color: var(--muted);
  }

  .share-notice {
    border-color: rgb(100 214 173 / 34%);
    background: #112a23;
    color: var(--green);
  }

  @keyframes grid-drift {
    from {
      background-position: 0 0, 0 0;
    }

    to {
      background-position: 88px 0, 0 72px;
    }
  }

  @keyframes signal-sweep {
    0%,
    100% {
      opacity: 0;
      transform: translateX(-18%);
    }

    38%,
    62% {
      opacity: 1;
    }

    72% {
      opacity: 0;
      transform: translateX(18%);
    }
  }

  @keyframes scanline {
    0%,
    100% {
      opacity: 0;
      transform: translateY(0);
    }

    35%,
    70% {
      opacity: 1;
    }

    80% {
      opacity: 0;
      transform: translateY(164px);
    }
  }

  @keyframes activity-run {
    0%,
    100% {
      opacity: 0.4;
      transform: scaleX(0.32);
    }

    50% {
      opacity: 1;
      transform: scaleX(1);
    }
  }

  @keyframes todo-pulse {
    0%,
    100% {
      opacity: 0.72;
      transform: translateX(0);
    }

    50% {
      opacity: 1;
      transform: translateX(3px);
    }
  }

  @keyframes panel-arrive {
    from {
      opacity: 0;
      transform: translateY(12px);
    }

    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @keyframes mark-float {
    0%,
    100% {
      transform: translateY(0);
    }

    50% {
      transform: translateY(-4px);
    }
  }

  @media (max-width: 520px) {
    .login-panel {
      padding: 28px 22px;
    }

    .app-header,
    .form,
    .item {
      padding: 12px;
    }

    .brand-lockup,
    .item-top,
    .share-panel-header,
    .share-footer {
      align-items: stretch;
      flex-direction: column;
    }

    .header-actions,
    .item-actions,
    .access-actions,
    .section-tools {
      width: 100%;
      justify-content: flex-start;
    }

    .header-actions span,
    .header-actions button,
    .item button,
    .access-actions button {
      width: 100%;
    }
  }

  /* Japandi theme */
  :global(:root) {
    color-scheme: light;
    --bg: #f3f0e8;
    --surface: #fffdf8;
    --surface-soft: #e6e4e0;
    --surface-warm: #dfcaba;
    --surface-panel: rgb(255 253 248 / 86%);
    --surface-raised: rgb(255 253 248 / 92%);
    --surface-field: rgb(255 253 248 / 96%);
    --surface-muted: rgb(243 240 232 / 72%);
    --ink: #383633;
    --heading: #302f2b;
    --muted: #6f6963;
    --faint: #858480;
    --line: #d8d1c7;
    --line-strong: #b9b09f;
    --accent: #606c5a;
    --accent-strong: #475242;
    --accent-soft: #e4e8dc;
    --accent-muted: #b0b9a8;
    --wood: #dcb482;
    --wood-strong: #a87949;
    --clay: #c09e85;
    --rose: #e0cfc3;
    --danger: #9b4337;
    --danger-soft: #f4ded8;
    --button-bg: #606c5a;
    --button-bg-hover: #4f5b4a;
    --button-ink: #fffdf8;
    --glass-border: rgb(255 253 248 / 70%);
    --shadow: 0 22px 48px rgb(83 73 61 / 14%);
    --shadow-soft: 0 12px 28px rgb(83 73 61 / 9%);
    --focus-ring: 0 0 0 3px rgb(96 108 90 / 22%);
    --bamboo-line: rgb(96 108 90 / 20%);
    --body-overlay-a: linear-gradient(90deg, rgb(243 240 232 / 92%), rgb(243 240 232 / 64%) 42%, rgb(176 185 168 / 34%));
    --body-overlay-b: linear-gradient(180deg, rgb(255 253 248 / 82%), rgb(230 228 224 / 56%) 56%, rgb(176 185 168 / 44%));
  }

  :global(html[data-theme='dark']) {
    color-scheme: dark;
    --bg: #1d211c;
    --surface: #292c27;
    --surface-soft: #33372f;
    --surface-warm: #3a302a;
    --surface-panel: rgb(42 45 39 / 88%);
    --surface-raised: rgb(31 34 30 / 92%);
    --surface-field: rgb(23 26 23 / 94%);
    --surface-muted: rgb(51 55 47 / 74%);
    --ink: #f3f0e8;
    --heading: #fffaf2;
    --muted: #c8c0b4;
    --faint: #98a08f;
    --line: #4b5046;
    --line-strong: #646a5d;
    --accent: #b0b9a8;
    --accent-strong: #dcb482;
    --accent-soft: #354131;
    --accent-muted: #606c5a;
    --wood: #dcb482;
    --wood-strong: #e0c07e;
    --clay: #c09e85;
    --rose: #4b3933;
    --danger: #f09a8f;
    --danger-soft: #4a2724;
    --button-bg: #b0b9a8;
    --button-bg-hover: #dcb482;
    --button-ink: #20231f;
    --glass-border: rgb(255 250 242 / 11%);
    --shadow: 0 26px 58px rgb(0 0 0 / 34%);
    --shadow-soft: 0 14px 34px rgb(0 0 0 / 24%);
    --focus-ring: 0 0 0 3px rgb(176 185 168 / 23%);
    --bamboo-line: rgb(176 185 168 / 18%);
    --body-overlay-a: linear-gradient(90deg, rgb(29 33 28 / 95%), rgb(29 33 28 / 82%) 46%, rgb(32 35 30 / 78%));
    --body-overlay-b: linear-gradient(180deg, rgb(29 33 28 / 74%), rgb(20 23 20 / 92%) 68%, rgb(18 20 18 / 96%));
  }

  :global(html) {
    background: var(--bg);
  }

  :global(body) {
    background-color: var(--bg);
    background-image:
      var(--body-overlay-a),
      var(--body-overlay-b),
      url('./assets/bamboo-forest-japandi.png');
    background-position: center;
    background-repeat: no-repeat;
    background-size: cover;
    background-attachment: fixed;
    color: var(--ink);
  }

  .app,
  .login {
    position: relative;
    z-index: 0;
  }

  .login {
    width: 100%;
    min-height: 100vh;
    overflow: hidden;
    padding: clamp(28px, 6vw, 72px) 16px;
  }

  .login-backdrop {
    position: fixed;
    inset: 0;
    z-index: -1;
    overflow: hidden;
    pointer-events: none;
  }

  .login-backdrop::before {
    position: absolute;
    inset: 0;
    content: "";
    animation: none;
    background:
      linear-gradient(90deg, transparent 0 7%, var(--bamboo-line) 7% calc(7% + 1px), transparent calc(7% + 1px) 100%),
      linear-gradient(90deg, transparent 0 83%, var(--bamboo-line) 83% calc(83% + 1px), transparent calc(83% + 1px) 100%);
    mask-image: none;
    opacity: 0.58;
    transform: none;
  }

  .login-backdrop span {
    position: absolute;
    top: auto;
    bottom: -18vh;
    display: block;
    width: clamp(28px, 4vw, 52px);
    height: 76vh;
    border: 1px solid rgb(255 253 248 / 20%);
    border-radius: 8px;
    animation: none;
    background:
      linear-gradient(90deg, rgb(255 253 248 / 16%), transparent 34% 62%, rgb(0 0 0 / 10%)),
      linear-gradient(180deg, transparent 0 14%, var(--bamboo-line) 14% calc(14% + 2px), transparent calc(14% + 2px) 31%, var(--bamboo-line) 31% calc(31% + 2px), transparent calc(31% + 2px) 48%, var(--bamboo-line) 48% calc(48% + 2px), transparent calc(48% + 2px) 65%, var(--bamboo-line) 65% calc(65% + 2px), transparent calc(65% + 2px));
    opacity: 0.36;
    transform: rotate(-4deg);
  }

  .login-backdrop span:nth-child(1) {
    left: 4vw;
  }

  .login-backdrop span:nth-child(2) {
    right: 10vw;
    height: 68vh;
    opacity: 0.3;
    transform: rotate(3deg);
  }

  .login-backdrop span:nth-child(3) {
    right: 3vw;
    height: 88vh;
    opacity: 0.26;
    transform: rotate(-2deg);
  }

  .login-shell {
    width: min(1060px, 100%);
    min-height: min(680px, calc(100vh - 56px));
    grid-template-columns: minmax(0, 1fr) minmax(300px, 360px);
    gap: clamp(16px, 3vw, 28px);
    align-items: end;
  }

  .login-copy,
  .login-panel,
  .app-header,
  .form,
  .item,
  .share-panel {
    border-color: var(--glass-border);
    background: var(--surface-panel);
    box-shadow: var(--shadow);
    backdrop-filter: blur(16px) saturate(112%);
  }

  .login-copy {
    position: relative;
    min-height: 500px;
    padding: clamp(22px, 4vw, 36px);
    background:
      linear-gradient(135deg, rgb(255 253 248 / 88%), rgb(230 228 224 / 62) 48%, rgb(176 185 168 / 58)),
      var(--surface-panel);
  }

  .login-copy .brand-lockup {
    position: relative;
    z-index: 1;
    align-items: flex-start;
  }

  .brand-copy {
    max-width: 34rem;
    margin-top: 12px;
    color: var(--muted);
    font-size: clamp(1rem, 1.7vw, 1.18rem);
    font-weight: 620;
    line-height: 1.55;
  }

  :global(html[data-theme='dark']) .login-copy {
    background:
      linear-gradient(135deg, rgb(42 45 39 / 88%), rgb(31 34 30 / 78) 52%, rgb(54 65 49 / 72)),
      var(--surface-panel);
  }

  .login-copy::after {
    display: none;
  }

  .panda-art {
    position: relative;
    z-index: 1;
    width: min(100%, 680px);
    margin: 22px 0 0;
    overflow: hidden;
    border: 1px solid rgb(255 253 248 / 44%);
    border-radius: 8px;
    background:
      linear-gradient(135deg, rgb(255 253 248 / 54%), rgb(176 185 168 / 18));
    box-shadow: var(--shadow-soft);
  }

  .panda-art img {
    display: block;
    width: 100%;
    aspect-ratio: 4 / 3;
    object-fit: cover;
  }

  :global(html[data-theme='dark']) .panda-art {
    border-color: rgb(255 250 242 / 14%);
    background: rgb(31 34 30 / 72%);
  }

  .login-panel {
    gap: 16px;
    padding: 28px;
    animation: none;
  }

  .login-panel h2 {
    max-width: 18rem;
    font-size: clamp(1.65rem, 2.4vw, 2rem);
    line-height: 1.12;
  }

  .login-panel-copy {
    color: var(--muted);
    font-size: 0.96rem;
    font-weight: 620;
    line-height: 1.55;
  }

  .login-panel-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    margin-bottom: 10px;
  }

  .login-panel .brand-mark {
    width: 54px;
    height: 54px;
    margin: 0;
    animation: none;
    font-size: 1.18rem;
  }

  .brand-mark {
    border-color: rgb(96 108 90 / 34%);
    background:
      linear-gradient(135deg, var(--accent), var(--accent-strong));
    color: var(--button-ink);
    box-shadow: 0 12px 24px rgb(96 108 90 / 16%);
  }

  :global(html[data-theme='dark']) .brand-mark {
    color: #20231f;
  }

  .theme-slider {
    position: relative;
    isolation: isolate;
    display: grid;
    width: 132px;
    min-height: 38px;
    grid-template-columns: 1fr 1fr;
    align-items: center;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--surface-muted);
    color: var(--muted);
    cursor: pointer;
    font-size: 0.76rem;
    font-weight: 850;
    letter-spacing: 0;
    padding: 3px;
    user-select: none;
  }

  .theme-slider input {
    position: absolute;
    inset: 0;
    z-index: 4;
    width: 100%;
    height: 100%;
    min-height: 0;
    cursor: pointer;
    opacity: 0;
  }

  .theme-slider span {
    position: relative;
    z-index: 3;
    display: grid;
    min-width: 0;
    min-height: 30px;
    place-items: center;
    line-height: 1;
    transition: color 140ms ease;
  }

  .theme-slider i {
    position: absolute;
    inset: 3px auto 3px 3px;
    z-index: 2;
    width: calc(50% - 3px);
    border-radius: 6px;
    background: var(--surface);
    box-shadow: var(--shadow-soft);
    transition: transform 160ms ease, background-color 160ms ease;
  }

  .theme-slider input:not(:checked) ~ span:first-of-type,
  .theme-slider input:checked ~ span:nth-of-type(2) {
    color: var(--heading);
  }

  .theme-slider input:checked ~ i {
    transform: translateX(100%);
  }

  .theme-slider:focus-within {
    box-shadow: var(--focus-ring);
  }

  .app {
    width: min(1180px, calc(100vw - 32px));
    padding: 28px 0 56px;
  }

  .app-header {
    background: var(--surface-panel);
  }

  .header-actions {
    gap: 10px;
  }

  .header-actions span,
  .count,
  .meta-line span,
  .permission-help {
    border-color: var(--line);
    background: var(--surface-muted);
    color: var(--muted);
  }

  h1,
  h2,
  .header-actions strong,
  .item strong {
    color: var(--heading);
  }

  .eyebrow {
    color: var(--accent);
  }

  .note-form,
  .todo-form,
  .share-panel {
    border-color: var(--glass-border);
    background: var(--surface-panel);
  }

  input,
  select,
  textarea {
    border-color: var(--line-strong);
    background: var(--surface-field);
    color: var(--ink);
  }

  input:focus,
  select:focus,
  textarea:focus {
    border-color: var(--accent);
    box-shadow: var(--focus-ring);
  }

  input::placeholder,
  textarea::placeholder {
    color: var(--faint);
  }

  button:focus-visible {
    outline: 3px solid rgb(96 108 90 / 24%);
    outline-offset: 2px;
  }

  .primary-action {
    background: var(--button-bg);
    color: var(--button-ink);
    box-shadow: 0 12px 22px rgb(96 108 90 / 18%);
  }

  .primary-action:not(:disabled):hover {
    background: var(--button-bg-hover);
    box-shadow: 0 14px 26px rgb(96 108 90 / 20%);
  }

  .ghost {
    border-color: var(--line-strong);
    background: var(--surface-raised);
    color: var(--ink);
  }

  .ghost:not(:disabled):hover {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--heading);
  }

  .item {
    border-color: var(--line);
    background: var(--surface-raised);
  }

  .item:hover {
    border-color: var(--accent-muted);
    box-shadow: var(--shadow);
  }

  .note-card p,
  .user-copy small {
    color: var(--muted);
  }

  .completed {
    background: var(--surface-muted);
  }

  .priority-low {
    border-color: rgb(96 108 90 / 34%);
    background: var(--accent-soft);
    color: var(--accent-strong);
  }

  .priority-medium {
    border-color: rgb(220 180 130 / 50%);
    background: rgb(220 180 130 / 18%);
    color: var(--wood-strong);
  }

  .priority-high,
  .danger:not(:disabled):hover,
  .error {
    border-color: rgb(155 67 55 / 34%);
    background: var(--danger-soft);
    color: var(--danger);
  }

  .readonly-access {
    border-color: rgb(220 180 130 / 48%) !important;
    background: rgb(220 180 130 / 18%) !important;
    color: var(--wood-strong) !important;
  }

  .user-option,
  .switch-field {
    border-color: var(--line);
    background: var(--surface-field);
    color: var(--ink);
  }

  .user-option:not(:disabled):hover,
  .selected-user {
    border-color: var(--accent);
    background: var(--accent-soft);
    box-shadow: var(--shadow-soft);
  }

  .user-avatar {
    background: var(--accent-soft);
    color: var(--accent-strong);
  }

  .share-notice {
    border-color: rgb(96 108 90 / 34%);
    background: var(--accent-soft);
    color: var(--accent-strong);
  }

  .switch-field input {
    accent-color: var(--accent);
  }

  .todo-title input {
    accent-color: var(--accent);
  }

  .calendar-synced {
    border-color: rgb(96 108 90 / 44%) !important;
    background: var(--accent-soft) !important;
    color: var(--accent-strong) !important;
    font-weight: 850;
  }

  .calendar-error {
    border-color: rgb(155 67 55 / 34%) !important;
    background: var(--danger-soft) !important;
    color: var(--danger) !important;
    font-weight: 850;
  }

  .calendar-off {
    border-color: var(--line) !important;
    background: var(--surface-muted) !important;
    color: var(--muted) !important;
  }

  .todo-card {
    gap: 12px;
    padding: 16px;
  }

  .todo-card .item-top {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    align-items: stretch;
    gap: 14px;
  }

  .todo-card .item-content {
    display: grid;
    gap: 10px;
  }

  .todo-card .todo-title {
    display: grid;
    grid-template-columns: 22px minmax(0, 1fr);
    align-items: start;
    gap: 12px;
  }

  .todo-card .todo-title input {
    margin-top: 1px;
  }

  .todo-card .todo-title strong {
    font-size: 1.04rem;
    line-height: 1.35;
    overflow-wrap: break-word;
    word-break: normal;
  }

  .todo-card .meta-line {
    display: flex;
    margin-top: 0;
    padding-left: 34px;
  }

  .todo-card .meta-line span {
    max-width: 100%;
    white-space: normal;
    word-break: normal;
  }

  .todo-card .item-actions {
    width: 100%;
    justify-content: flex-start;
    padding-left: 34px;
  }

  .todo-card .item-actions .small-button {
    min-width: 72px;
  }

  .todo-card .calendar-button {
    min-width: 128px;
  }

  @media (max-width: 820px) {
    .app,
    .login {
      width: min(100% - 20px, 1120px);
    }

    .login {
      width: 100%;
      padding: 20px 10px;
    }

    .login-shell {
      min-height: auto;
      grid-template-columns: 1fr;
    }

    .login-copy {
      min-height: 360px;
    }

    .app-header,
    .content-grid {
      grid-template-columns: 1fr;
    }

    .app-header {
      align-items: flex-start;
      flex-direction: column;
    }

    .content-grid {
      gap: 16px;
    }

    .todo-form,
    .todo-edit-grid {
      grid-template-columns: 1fr;
    }

    .todo-card .meta-line,
    .todo-card .item-actions {
      padding-left: 0;
    }
  }

  @media (max-width: 520px) {
    .login-panel {
      padding: 24px 20px;
    }

    .login-panel-top {
      align-items: flex-start;
      flex-direction: column;
    }

    .theme-slider {
      width: 100%;
    }

    .app-header,
    .form,
    .item {
      padding: 12px;
    }

    .brand-lockup,
    .item-top,
    .share-panel-header,
    .share-footer {
      align-items: stretch;
      flex-direction: column;
    }

    .header-actions,
    .item-actions,
    .access-actions,
    .section-tools {
      width: 100%;
      justify-content: flex-start;
    }

    .header-actions span,
    .header-actions button,
    .header-actions .theme-slider,
    .item button,
    .access-actions button {
      width: 100%;
    }

    .todo-card .item-actions .small-button,
    .todo-card .calendar-button {
      min-width: 0;
    }
  }

  .todo-card > .item-top {
    display: grid !important;
    grid-template-columns: minmax(0, 1fr) !important;
    align-items: stretch;
  }

  .todo-card .item-content,
  .todo-card .todo-title,
  .todo-card .meta-line,
  .todo-card .item-actions {
    min-width: 0;
    width: 100%;
  }

  .todo-card .todo-title {
    display: grid !important;
    grid-template-columns: 22px minmax(0, 1fr);
    align-items: start;
  }

  .todo-card .todo-title strong {
    display: block;
    min-width: 0;
    width: 100%;
    overflow-wrap: break-word;
    word-break: normal;
  }

  .todo-card .meta-line {
    display: flex !important;
    flex-wrap: wrap;
    align-items: flex-start;
  }

  .todo-card .meta-line span {
    flex: 0 1 auto;
    width: auto;
    max-width: 100%;
    overflow-wrap: break-word;
    word-break: normal;
  }

  .todo-card .item-actions {
    display: flex !important;
    flex-wrap: wrap;
    justify-content: flex-start;
  }
</style>
