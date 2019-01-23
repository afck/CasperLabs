package io.casperlabs.shared

import java.nio.file._
import java.nio.file.attribute.BasicFileAttributes

import cats.effect.Sync
import cats.implicits._

object PathOps {

  implicit class RichPath(value: Path) {
    def folderSize: Long =
      Files
        .walk(value)
        .mapToLong(p => {
          val f = p.toFile
          if (f.isFile)
            f.length
          else
            0
        })
        .sum()

    def recursivelyDelete(): Path =
      Files.walkFileTree(value, makeDeleteFileVisitor)
  }

  /**
    * Makes a SimpleFileVisitor to delete files and the directories that contained them
    *
    * [[https://docs.oracle.com/javase/8/docs/api/java/nio/file/FileVisitor.html]]
    */
  private def makeDeleteFileVisitor: SimpleFileVisitor[Path] =
    new SimpleFileVisitor[Path] {
      override def visitFile(p: Path, attrs: BasicFileAttributes): FileVisitResult = {
        Files.delete(p)
        FileVisitResult.CONTINUE
      }
      override def postVisitDirectory(p: Path, e: java.io.IOException): FileVisitResult = {
        Files.delete(p)
        FileVisitResult.CONTINUE
      }
    }

  implicit class PathDelete(path: Path) {
    def deleteSingleFile[F[_]: Sync: Log]()(implicit logSource: LogSource): F[Unit] = {

      def delete(): F[Unit] =
        for {
          result <- Sync[F].delay(path.toFile.delete).attempt
          _ <- result match {
                case Left(t) =>
                  Log[F].error(s"Can't delete file $path: ${t.getMessage}", t)
                case Right(false) =>
                  Log[F].warn(s"Can't delete file $path.")
                case Right(true) =>
                  Log[F].debug(s"Deleted file $path")
              }
        } yield ()

      for {
        _ <- Sync[F]
              .delay(path.toFile.exists)
              .ifM(delete(), Log[F].warn(s"Can't delete file $path. File not found."))
      } yield ()
    }

    def deleteDirectory[F[_]: Sync: Log]()(implicit logSource: LogSource): F[Unit] = {
      import java.io.File
      import java.util.Comparator

      import cats.implicits._

      import scala.collection.JavaConverters._

      def deleteFile(file: File): F[Unit] =
        for {
          _ <- Sync[F]
                .delay(file.delete)
                .ifM(
                  Log[F].info(s"Deleted file ${file.getAbsolutePath}"),
                  Log[F].warn(s"Can't delete file ${file.getAbsolutePath}")
                )
        } yield ()

      def getFiles: F[List[Path]] = Sync[F].delay(
        Files
          .walk(path)
          .sorted(Comparator.reverseOrder())
          .iterator()
          .asScala
          .toList
      )

      def delete0(): F[Unit] =
        for {
          files <- getFiles
          _     <- files.traverse(p => deleteFile(p.toFile))
        } yield ()

      for {
        result <- delete0().attempt
        _ <- result match {
              case Left(_: NoSuchFileException) =>
                Log[F].warn(s"Can't delete file or directory $path: No such file")
              case Left(t) =>
                Log[F].error(s"Can't delete file or directory $path: ${t.getMessage}", t)
              case Right(_) => ().pure[F]
            }
      } yield ()
    }
  }
}