//! The program's whole process tree, ended at once: a process group on Unix, a Job object on Windows. Every group
//! the runner holds is listed while its program runs, so the host's own SIGINT or SIGTERM ([`stop_on_interrupt`])
//! ends them too: a group no longer shares the terminal's signals.

use std::process::{Child, Command};

/// How many groups the live list holds. shortcut: a run past this many at once is not ended by an interrupt (it still
/// ends at its deadline); upgrade when the host runs that many processes concurrently.
const LIVE: usize = 64;

/// Start the program as the leader of its own process group, so the group can be ended with it.
#[cfg(unix)]
pub fn isolate(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

/// Start the program suspended: it is put in its Job object before it runs, so no process it starts escapes the Job.
#[cfg(windows)]
pub fn isolate(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    command.creation_flags(windows_sys::Win32::System::Threading::CREATE_SUSPENDED);
}

#[cfg(unix)]
mod live {
    use std::sync::atomic::{AtomicI32, Ordering};

    /// The process group ids of the running programs; 0 is a free slot.
    static GROUPS: [AtomicI32; super::LIVE] = [const { AtomicI32::new(0) }; super::LIVE];

    pub fn hold(pgid: i32) -> Option<usize> {
        GROUPS.iter().position(|slot| {
            slot.compare_exchange(0, pgid, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
        })
    }

    pub fn release(slot: usize) {
        GROUPS[slot].store(0, Ordering::Release);
    }

    /// Kill every listed group: only atomics and kill(2), so a signal handler may call it.
    pub fn stop_all() {
        for slot in &GROUPS {
            let pgid = slot.swap(0, Ordering::AcqRel);
            if pgid > 0 {
                // SAFETY: kill(2) takes plain integers and is async-signal-safe.
                #[expect(unsafe_code, reason = "kill(2) on a process group the runner created")]
                unsafe {
                    libc::kill(-pgid, libc::SIGKILL);
                }
            }
        }
    }

    pub fn count() -> usize {
        GROUPS
            .iter()
            .filter(|slot| slot.load(Ordering::Acquire) != 0)
            .count()
    }
}

#[cfg(windows)]
mod live {
    use std::sync::atomic::{AtomicPtr, Ordering};

    /// The Job objects of the running programs; null is a free slot. The handle stays the group's: it is closed on
    /// drop, after its slot is released.
    static JOBS: [AtomicPtr<core::ffi::c_void>; super::LIVE] =
        [const { AtomicPtr::new(std::ptr::null_mut()) }; super::LIVE];

    pub fn hold(job: *mut core::ffi::c_void) -> Option<usize> {
        JOBS.iter().position(|slot| {
            slot.compare_exchange(
                std::ptr::null_mut(),
                job,
                Ordering::AcqRel,
                Ordering::Relaxed,
            )
            .is_ok()
        })
    }

    pub fn release(slot: usize) {
        JOBS[slot].store(std::ptr::null_mut(), Ordering::Release);
    }

    /// End every listed Job's processes.
    pub fn stop_all() {
        for slot in &JOBS {
            let job = slot.load(Ordering::Acquire);
            if !job.is_null() {
                // SAFETY: a listed handle is open until its group releases the slot, then closes it.
                #[expect(unsafe_code, reason = "TerminateJobObject on a Job the runner created")]
                unsafe {
                    windows_sys::Win32::System::JobObjects::TerminateJobObject(job, 130);
                }
            }
        }
    }

    pub fn count() -> usize {
        JOBS.iter()
            .filter(|slot| !slot.load(Ordering::Acquire).is_null())
            .count()
    }
}

/// End every process group (or Job) the runner holds now. What the host's interrupt handler calls.
pub fn stop_all() {
    live::stop_all();
}

/// How many groups the runner holds now.
pub fn live() -> usize {
    live::count()
}

/// On the host's SIGINT or SIGTERM, end every group the runner holds, then die of the signal as before.
#[cfg(unix)]
pub fn stop_on_interrupt() {
    extern "C" fn on_signal(signal: libc::c_int) {
        live::stop_all();
        // SAFETY: signal(2) and raise(3) are async-signal-safe; the default action ends the host as the signal would.
        #[expect(unsafe_code, reason = "restoring the default action and re-raising")]
        unsafe {
            libc::signal(signal, libc::SIG_DFL);
            libc::raise(signal);
        }
    }
    for signal in [libc::SIGINT, libc::SIGTERM] {
        // SAFETY: the handler only touches atomics and async-signal-safe calls.
        #[expect(unsafe_code, reason = "installing the interrupt handler")]
        unsafe {
            libc::signal(
                signal,
                on_signal as extern "C" fn(libc::c_int) as libc::sighandler_t,
            );
        }
    }
}

/// On the host's Ctrl-C or Ctrl-Break (or its console closing), end every Job the runner holds, then let the default
/// handler end the host.
#[cfg(windows)]
pub fn stop_on_interrupt() {
    extern "system" fn on_control(_kind: u32) -> windows_sys::Win32::Foundation::BOOL {
        live::stop_all();
        0
    }
    // SAFETY: the handler is a plain function that touches atomics and the Job handles only.
    #[expect(unsafe_code, reason = "installing the console control handler")]
    unsafe {
        let handler: unsafe extern "system" fn(u32) -> windows_sys::Win32::Foundation::BOOL =
            on_control;
        windows_sys::Win32::System::Console::SetConsoleCtrlHandler(Some(handler), 1);
    }
}

/// The process group a program leads, listed while it runs.
#[cfg(unix)]
pub struct Group {
    pgid: libc::pid_t,
    slot: Option<usize>,
}

#[cfg(unix)]
impl Group {
    /// The group `child` leads, listed live. Never fails on Unix: the group exists from the spawn.
    pub fn of(child: &mut Child) -> Result<Self, String> {
        let pgid = libc::pid_t::try_from(child.id()).unwrap_or(0);
        let slot = (pgid > 0).then(|| live::hold(pgid)).flatten();
        Ok(Group { pgid, slot })
    }

    /// The program exited: its group is no longer listed (its id may soon be another's).
    pub fn release(&mut self) {
        if let Some(slot) = self.slot.take() {
            live::release(slot);
        }
    }

    /// End every process of the group, then the program itself. Only while the program has not been reaped: past
    /// that the group id may be another's.
    pub fn kill(&mut self, child: &mut Child) {
        if self.pgid > 0 {
            // SAFETY: kill(2) takes plain integers; a negative pid names the group this runner made the child lead.
            #[expect(
                unsafe_code,
                reason = "kill(2) on the process group the runner created; no memory is shared"
            )]
            unsafe {
                libc::kill(-self.pgid, libc::SIGKILL);
            }
        }
        let _ = child.kill();
        self.release();
    }
}

#[cfg(unix)]
impl Drop for Group {
    fn drop(&mut self) {
        self.release();
    }
}

/// A Job object that holds the program and every process it starts; closing it ends them all.
#[cfg(windows)]
pub struct Group {
    job: windows_sys::Win32::Foundation::HANDLE,
    slot: Option<usize>,
}

#[cfg(windows)]
#[expect(
    unsafe_code,
    reason = "the Win32 Job object and thread calls; each handle is owned by this value and closed once"
)]
impl Group {
    /// Put the suspended `child` in a new Job object that kills it all when closed, then let it run. Any step that
    /// fails ends the child: a program the Job does not hold is not run.
    pub fn of(child: &mut Child) -> Result<Self, String> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            SetInformationJobObject,
        };
        let refuse = |child: &mut Child, step: &str| {
            let error = std::io::Error::last_os_error();
            let _ = child.kill();
            let _ = child.wait();
            format!("the Job object that holds it could not be made ({step}: {error})")
        };
        // SAFETY: a null security descriptor and name are allowed; the handle is checked before use.
        let job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if job.is_null() {
            return Err(refuse(child, "CreateJobObjectW"));
        }
        let mut group = Group { job, slot: None };
        // SAFETY: a zeroed limit structure is a valid "no limit" value, and its size is passed with it.
        let limited = unsafe {
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                std::ptr::from_ref(&limits).cast(),
                u32::try_from(std::mem::size_of_val(&limits)).unwrap_or(0),
            )
        };
        if limited == 0 {
            return Err(refuse(child, "SetInformationJobObject"));
        }
        // SAFETY: both handles are open: the job here, the process while `child` lives.
        if unsafe { AssignProcessToJobObject(job, child.as_raw_handle().cast()) } == 0 {
            return Err(refuse(child, "AssignProcessToJobObject"));
        }
        group.slot = live::hold(job);
        if let Err(step) = resume(child.id()) {
            return Err(refuse(child, step));
        }
        Ok(group)
    }

    /// The program exited: its Job is no longer listed.
    pub fn release(&mut self) {
        if let Some(slot) = self.slot.take() {
            live::release(slot);
        }
    }

    /// End every process of the job, then the program itself.
    pub fn kill(&mut self, child: &mut Child) {
        // SAFETY: the job handle is open until drop.
        unsafe {
            windows_sys::Win32::System::JobObjects::TerminateJobObject(self.job, 1);
        }
        let _ = child.kill();
        self.release();
    }
}

/// Resume every thread of the suspended process `pid` (its one main thread).
#[cfg(windows)]
#[expect(
    unsafe_code,
    reason = "the Toolhelp thread walk; the snapshot and each thread handle are closed here"
)]
fn resume(pid: u32) -> Result<(), &'static str> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
    };
    use windows_sys::Win32::System::Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME};
    // SAFETY: the snapshot handle is checked, then closed once below.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err("CreateToolhelp32Snapshot");
    }
    let mut resumed = false;
    // SAFETY: the entry's size is set as the API asks; each opened thread handle is closed after use.
    unsafe {
        let mut entry: THREADENTRY32 = std::mem::zeroed();
        entry.dwSize = u32::try_from(std::mem::size_of::<THREADENTRY32>()).unwrap_or(0);
        let mut more = Thread32First(snapshot, &mut entry) != 0;
        while more {
            if entry.th32OwnerProcessID == pid {
                let thread = OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID);
                if !thread.is_null() {
                    resumed |= ResumeThread(thread) != u32::MAX;
                    CloseHandle(thread);
                }
            }
            more = Thread32Next(snapshot, &mut entry) != 0;
        }
        CloseHandle(snapshot);
    }
    if resumed { Ok(()) } else { Err("ResumeThread") }
}

#[cfg(windows)]
impl Drop for Group {
    #[expect(unsafe_code, reason = "closing the job handle this value owns")]
    fn drop(&mut self) {
        self.release();
        // SAFETY: the handle is owned here, no longer listed, and closed once.
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.job);
        }
    }
}
