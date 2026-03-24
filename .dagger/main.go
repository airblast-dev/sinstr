// A generated module for Sinstr functions
//
// This module has been generated via dagger init and serves as a reference to
// basic module structure as you get started with Dagger.
//
// Two functions have been pre-created. You can modify, delete, or add to them,
// as needed. They demonstrate usage of arguments and return types using simple
// echo and grep commands. The functions can be called from the dagger CLI or
// from one of the SDKs.
//
// The first line in this comment block is a short description line and the
// rest is a long description with more detail on the module's purpose or usage,
// if appropriate. All modules should have a short description.

package main

import (
	"context"
	"dagger/sinstr/internal/dagger"
)

type Sinstr struct{}



func (m *Sinstr) TestFat(ctx context.Context, 
// +defaultPath="/"
src *dagger.Directory) (string, error) {
	return dag.Container().
		From("rust:latest").
		WithExec([]string{"cargo", "install", "just"}).
		WithMountedDirectory("/mnt", src).
		WithWorkdir("/mnt").
		WithExec([]string{"just", "all"}).
		Stdout(ctx)
}
