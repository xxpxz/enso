package org.enso.compiler.pass.resolve

import org.enso.compiler.context.{InlineContext, ModuleContext}
import org.enso.compiler.core.IR
import org.enso.compiler.core.IR.Module.Scope.Definition
import org.enso.compiler.core.ir.MetadataStorage._
import org.enso.compiler.core.CompilerError
import org.enso.compiler.pass.IRPass

/** A pass responsible for the discovery of [[IR.Name.GenericAnnotation]]
  * annotations, and for associating them with the corresponding construct.
  *
  * Compilation pipeline of generic annotations:
  * - [[ModuleAnnotations]] pass ignores generic annotations and leaves them in
  * the tree so that the consequent passes are able to process the annotation
  * expression.
  * - [[org.enso.compiler.pass.desugar.ComplexType]] pass associates generic
  * annotations with the type constructor definitions.
  * - [[GenericAnnotations]] pass associates generic annotations that are left
  * in the tree with the appropriate definitions.
  */
case object GenericAnnotations extends IRPass {
  override type Metadata = ModuleAnnotations.Annotations
  override type Config   = IRPass.Configuration.Default
  override val precursorPasses: Seq[IRPass]   = Seq()
  override val invalidatedPasses: Seq[IRPass] = Seq()

  /** Resolves annotations.
    *
    * @param ir the Enso IR to process
    * @param moduleContext a context object that contains the information needed
    * to process a module
    * @return `ir`, possibly having made transformations or annotations to that
    * IR.
    */
  override def runModule(
    ir: IR.Module,
    moduleContext: ModuleContext
  ): IR.Module = {
    var lastAnnotations: Seq[IR.Name.GenericAnnotation] = Seq()
    val newBindings = ir.bindings.map {
      case _: IR.Name.BuiltinAnnotation =>
        throw new CompilerError(
          s"Builtin annotations should not be present at generic annotations pass."
        )
      case _: Definition.SugaredType =>
        throw new CompilerError(
          s"Sugared types should not be present at generic annotations pass."
        )
      case _: IR.Comment =>
        throw new CompilerError(
          "Comments should not be present at generic annotations pass."
        )
      case ann: IR.Name.GenericAnnotation =>
        lastAnnotations :+= ann
        None
      case entity =>
        val res = Some(
          entity.updateMetadata(
            this -->> ModuleAnnotations.Annotations(lastAnnotations)
          )
        )
        lastAnnotations = Seq()
        res
    }
    ir.copy(bindings = newBindings.flatten)
  }

  /** Execute the pass on an expression.
    *
    * As the pass only deals with module-level annotations this is a no-op.
    *
    * @param ir the Enso IR to process
    * @param inlineContext a context object that contains the information needed
    *                      for inline evaluation
    *  @return `ir`, possibly having made transformations or annotations to that
    *         IR.
    */
  override def runExpression(
    ir: IR.Expression,
    inlineContext: InlineContext
  ): IR.Expression = ir

  /** @inheritdoc */

}
