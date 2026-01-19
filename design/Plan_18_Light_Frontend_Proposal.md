# Plan 18: Light Frontend Proposal for Non-Terminal Users

## Goals
- Provide a lightweight web-based interface for users who prefer GUI over CLI.
- Maintain simplicity and minimal dependencies.
- Reuse existing database and core logic.
- Focus on core task management workflows.

## Non-Goals
- Full-featured web application with all CLI capabilities.
- Real-time collaboration or multi-user support.
- Mobile app or native desktop application.
- Complex UI frameworks or heavy dependencies.

## Assumptions
- Users have a modern web browser (Chrome, Firefox, Safari, Edge).
- Frontend runs locally (localhost) for security and simplicity.
- Database remains SQLite, accessed via REST API.
- CLI remains the primary interface; web UI is complementary.

## Design Overview

### Architecture
```
┌─────────────┐
│   Browser   │  (HTML/CSS/JavaScript)
└──────┬──────┘
       │ HTTP/REST
┌──────▼──────┐
│ Web Server  │  (Rust HTTP server)
└──────┬──────┘
       │
┌──────▼──────┐
│  Database   │  (SQLite, existing)
└─────────────┘
```

### Components
1. **Web Server** (`task serve` command)
   - Lightweight HTTP server (using `axum` or `actix-web`)
   - REST API endpoints for CRUD operations
   - Serves static HTML/CSS/JS files

2. **Frontend** (Static HTML/CSS/JavaScript)
   - Vanilla JavaScript (no framework dependencies)
   - Responsive design (works on desktop and tablet)
   - Minimal, clean UI focused on core workflows

3. **API Layer**
   - RESTful endpoints that wrap existing repository logic
   - JSON request/response format
   - Reuses existing database connection and models

---

## Core Features (MVP)

### 1. Task List View
- Display tasks in a table/card view
- Show: ID, Description, Status, Project, Tags, Due Date
- Basic filtering: by project, status, tags
- Sort by: ID, due date, status
- Click task to view details

### 2. Task Detail View
- Show full task information
- Edit task attributes (description, project, tags, due date)
- View annotations
- View sessions/time tracking
- Actions: Complete, Delete, Modify

### 3. Add Task
- Simple form: description, project, tags, due date
- Quick add (minimal fields) and full add (all fields)

### 4. Clock/Time Tracking
- Show current clock state (if running)
- Start/stop clock for task
- View clock stack
- Basic session history

### 5. Projects View
- List projects
- Create new project
- View tasks by project

---

## API Design

### Base URL
- `http://localhost:8080/api` (configurable port)

### Endpoints

#### Tasks
```
GET    /api/tasks                    # List tasks (with filters)
GET    /api/tasks/:id               # Get task details
POST   /api/tasks                   # Create task
PUT    /api/tasks/:id               # Update task
DELETE /api/tasks/:id               # Delete task
POST   /api/tasks/:id/annotate      # Add annotation
POST   /api/tasks/:id/finish        # Complete task
POST   /api/tasks/:id/close         # Close task
```

#### Projects
```
GET    /api/projects                # List projects
POST   /api/projects                # Create project
PUT    /api/projects/:id            # Update project
DELETE /api/projects/:id            # Delete/archive project
```

#### Clock/Sessions
```
GET    /api/clock/status            # Get clock state
POST   /api/clock/in/:task_id      # Start clock
POST   /api/clock/out               # Stop clock
GET    /api/clock/stack             # Get clock stack
POST   /api/clock/enqueue/:task_id  # Add to stack
GET    /api/sessions                # List sessions
GET    /api/sessions/:id            # Get session details
```

#### Filters/Query Parameters
- `?project=work` - Filter by project
- `?status=pending` - Filter by status
- `?tag=urgent` - Filter by tag
- `?search=bug` - Search description
- `?sort=due` - Sort column
- `?order=asc|desc` - Sort order

---

## Implementation Approach

### Phase 1: Web Server Foundation
1. Add HTTP server dependency (`axum` recommended - lightweight, async)
2. Create `task serve` command
3. Implement basic server with static file serving
4. Add health check endpoint

### Phase 2: API Layer
1. Create API module (`src/api/` or `src/web/`)
2. Implement task endpoints (CRUD)
3. Implement project endpoints
4. Implement clock/session endpoints
5. Add error handling and JSON responses

### Phase 3: Frontend
1. Create `web/` directory for static files
2. Build HTML structure (minimal, semantic)
3. Add CSS (simple, clean styling)
4. Implement JavaScript for API calls
5. Build task list view
6. Build task detail/edit view
7. Build add task form

### Phase 4: Integration & Polish
1. Connect frontend to API
2. Add error handling and user feedback
3. Test end-to-end workflows
4. Add basic responsive design
5. Documentation

---

## Technical Decisions

### Web Server Framework
**Choice: `axum`**
- Lightweight and modern
- Built on `tokio` (async)
- Good documentation
- Minimal dependencies
- Easy to integrate with existing code

**Alternative: `actix-web`**
- More features, but heavier
- Good for complex applications (overkill here)

### Frontend Framework
**Choice: Vanilla JavaScript**
- No build step required
- Minimal dependencies
- Easy to understand and modify
- Fast to load

**Alternative: Lightweight framework (Preact, Alpine.js)**
- Could add if vanilla JS becomes unwieldy
- Start simple, add complexity only if needed

### Static File Serving
- Embed static files in binary (using `include_dir` or similar)
- Or serve from `web/` directory relative to binary
- Prefer embedded for single-binary distribution

### API Authentication
**Choice: None (local only)**
- Server runs on localhost only
- No authentication needed for local access
- If remote access needed later, add basic auth or token

### CORS
- Not needed for localhost-only access
- If remote access added later, configure CORS appropriately

---

## File Structure

```
task-ninja/
├── src/
│   ├── api/              # New: API handlers
│   │   ├── mod.rs
│   │   ├── tasks.rs
│   │   ├── projects.rs
│   │   └── clock.rs
│   ├── cli/
│   │   └── commands.rs    # Add `task serve` command
│   └── ...
├── web/                  # New: Static frontend files
│   ├── index.html
│   ├── css/
│   │   └── style.css
│   ├── js/
│   │   ├── app.js
│   │   ├── api.js
│   │   └── views.js
│   └── assets/
└── Cargo.toml            # Add axum, tokio dependencies
```

---

## User Experience

### Starting the Server
```bash
# Start web server (default port 8080)
task serve

# Custom port
task serve --port 3000

# Custom host (if needed)
task serve --host 0.0.0.0 --port 8080
```

### Accessing the UI

#### Local Access
1. Run `task serve`
2. Open browser to `http://localhost:8080`
3. Use the web interface

#### Remote Access via SSH Port Forwarding
If you're connected to a remote computer via SSH, you can forward the port to access the web UI from your local browser:

```bash
# On remote computer: Start the server
ssh user@remote-server
task serve --port 8080

# On local computer: Forward the port (in a separate terminal)
ssh -L 8080:localhost:8080 user@remote-server

# Then open browser locally to http://localhost:8080
# The connection will be forwarded to the remote server
```

**Benefits:**
- Secure: Traffic is encrypted through SSH tunnel
- No need to expose server to network
- Works with default localhost-only binding
- No additional authentication needed (SSH handles it)

**Alternative: One-line port forwarding**
```bash
# Forward port and start server in one command (on remote)
ssh user@remote-server "task serve --port 8080" &
ssh -L 8080:localhost:8080 -N user@remote-server
```

### Workflow Example
1. User opens browser to `http://localhost:8080`
2. Sees task list (default view)
3. Clicks "Add Task" → fills form → submits
4. Task appears in list
5. Clicks task → sees details
6. Can edit, complete, or delete
7. Can start clock from task detail view

---

## Design Principles

### Simplicity
- Minimal UI elements
- Clear, uncluttered interface
- Focus on core workflows

### Consistency
- Match CLI behavior where possible
- Use same terminology (projects, tags, etc.)
- Same data model and constraints

### Performance
- Fast page loads (< 1 second)
- Efficient API responses
- Minimal JavaScript execution

### Accessibility
- Semantic HTML
- Keyboard navigation support
- Screen reader friendly (basic)

---

## Security Considerations

### Local-Only Access
- Server binds to `127.0.0.1` by default (localhost only)
- No external network access
- No authentication needed
- **SSH Port Forwarding**: Users can securely access remote servers via SSH tunnel (see "Accessing the UI" section)

### Input Validation
- Validate all API inputs
- Sanitize user-provided data
- Use existing CLI validation logic

### SQL Injection
- Use parameterized queries (already done via rusqlite)
- No raw SQL from API layer

### XSS Prevention
- Escape user input in HTML
- Use textContent instead of innerHTML where possible

---

## Future Enhancements (Out of Scope for MVP)

1. **Real-time Updates**
   - WebSocket support for live updates
   - Auto-refresh task list when data changes

2. **Advanced Filtering**
   - Complex filter builder UI
   - Save filter presets

3. **Keyboard Shortcuts**
   - Vim-like navigation
   - Quick actions (j/k for navigation, etc.)

4. **Themes**
   - Dark mode
   - Customizable colors

5. **Mobile Optimization**
   - Better responsive design
   - Touch-friendly interactions

6. **Offline Support**
   - Service worker for offline access
   - Local storage caching

7. **Export/Import**
   - Export tasks to JSON/CSV
   - Import from other tools

---

## Implementation Order

### Step 1: Proof of Concept
1. Add `axum` dependency
2. Create `task serve` command (minimal)
3. Serve a simple "Hello World" HTML page
4. Verify it works

### Step 2: Basic API
1. Implement `GET /api/tasks` endpoint
2. Return JSON list of tasks
3. Test with `curl` or browser

### Step 3: Task List UI
1. Create HTML page with task list
2. Fetch tasks from API
3. Display in table
4. Basic styling

### Step 4: Task Details
1. Add `GET /api/tasks/:id` endpoint
2. Create task detail page
3. Link from list to detail

### Step 5: Add/Edit Tasks
1. Add `POST /api/tasks` and `PUT /api/tasks/:id` endpoints
2. Create add/edit forms
3. Submit and update UI

### Step 6: Remaining Features
1. Projects view
2. Clock functionality
3. Annotations
4. Polish and error handling

---

## Dependencies to Add

```toml
[dependencies]
# Web server
axum = "0.7"
tokio = { version = "1", features = ["full"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["fs", "cors"] }

# Static file embedding (optional)
include_dir = "0.7"
```

---

## Testing Strategy

### API Testing
- Unit tests for API handlers
- Integration tests with test database
- Use `reqwest` or `hyper` for HTTP client testing

### Frontend Testing
- Manual testing in browsers
- Test core workflows end-to-end
- Test error cases (network errors, invalid input)

### Acceptance Testing
- Extend existing acceptance test framework
- Test web UI workflows
- Compare results with CLI commands

---

## Documentation Updates

1. **README.md**
   - Add "Web Interface" section
   - Document `task serve` command
   - Link to web UI usage

2. **COMMAND_REFERENCE.md**
   - Document `task serve` command
   - Document API endpoints (for developers)

3. **New: WEB_UI.md**
   - User guide for web interface
   - Screenshots/examples
   - Common workflows

---

## Risks and Mitigations

### Risk 1: Feature Creep
- **Mitigation**: Strictly limit MVP to core features. Defer advanced features.

### Risk 2: Performance with Large Datasets
- **Mitigation**: Add pagination to API endpoints. Limit results per page.

### Risk 3: Browser Compatibility
- **Mitigation**: Use modern JavaScript features, test in major browsers.

### Risk 4: Maintenance Burden
- **Mitigation**: Keep frontend simple. Reuse existing logic. Minimal custom code.

### Risk 5: Security Vulnerabilities
- **Mitigation**: Local-only access. Input validation. Regular dependency updates.

---

## Success Criteria

1. ✅ Users can view task list in browser
2. ✅ Users can add new tasks via web UI
3. ✅ Users can view and edit task details
4. ✅ Users can start/stop clock from web UI
5. ✅ All changes persist to same database as CLI
6. ✅ Web UI is responsive and usable
7. ✅ Server starts quickly (< 2 seconds)
8. ✅ Page loads quickly (< 1 second)

---

## Alternative Approaches Considered

### 1. Tauri Desktop App
- **Pros**: Native feel, direct database access
- **Cons**: Much heavier, requires build system, more complex

### 2. Electron Desktop App
- **Pros**: Cross-platform, web technologies
- **Cons**: Very heavy (100MB+), slower startup

### 3. Full-Stack Framework (React, Vue, etc.)
- **Pros**: Rich ecosystem, component reusability
- **Cons**: Build complexity, larger bundle size, overkill for simple UI

### 4. Server-Side Rendering (SSR)
- **Pros**: Faster initial load, SEO (not needed here)
- **Cons**: More complex, requires template engine

**Decision**: Simple static frontend + REST API is the lightest approach that meets requirements.

---

## Open Questions

1. **Port Configuration**: Should port be configurable via config file, or just command-line flag?
   - **Decision**: Command-line flag for MVP, config file later if needed.

2. **Auto-Open Browser**: Should `task serve` automatically open browser?
   - **Decision**: Optional `--open` flag.

3. **Multiple Instances**: Should we prevent multiple server instances?
   - **Decision**: Check if port is in use, show error if so.

4. **HTTPS**: Should we support HTTPS for local development?
   - **Decision**: No for MVP (localhost HTTP is fine). Add later if needed.

5. **API Versioning**: Should API be versioned (`/api/v1/`)?
   - **Decision**: Not needed for MVP. Add versioning if API changes significantly.

---

## Conclusion

This proposal provides a lightweight, practical web interface for task-ninja that:
- Maintains simplicity and minimal dependencies
- Reuses existing database and logic
- Focuses on core workflows
- Can be extended incrementally

The implementation is straightforward and can be built incrementally, starting with a proof of concept and expanding to full MVP functionality.
