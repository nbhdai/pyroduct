pub enum SessionResponse<T> {
    Continue(T),
    End
}